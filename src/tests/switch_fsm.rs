#[cfg(test)]
mod tests {
    use crate::{FsmState, FsmTransition};
    use std::{
        error, fmt,
        sync::{Arc, RwLock},
        thread,
    };

    // トレイト
    pub trait SendFsmError: error::Error + Send + 'static {}
    type ErrorType = dyn SendFsmError;
    type EventType = SwichEvent;
    type CtxType = Arc<RwLock<SwichContext>>;
    type StateDynType = dyn FsmState<Event = EventType, Ctx = CtxType, Error = Box<ErrorType>>;
    type StateBoxType = Box<StateDynType>;

    /**
     * Event Not Found Fsm エラー
     */

    // Event Not Found Fsm エラー 構造体
    #[derive(Debug)]
    pub struct EventNotFoundFsmError {
        pub event: EventType,
    }

    // Event Not Found Fsm エラー 実装
    impl EventNotFoundFsmError {
        fn new(event: SwichEvent) -> EventNotFoundFsmError {
            EventNotFoundFsmError { event }
        }
    }

    impl SendFsmError for EventNotFoundFsmError {}

    impl fmt::Display for EventNotFoundFsmError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "event not found: {:?}", &self.event)
        }
    }

    impl error::Error for EventNotFoundFsmError {
        fn source(&self) -> Option<&(dyn error::Error + 'static)> {
            None
        }
    }

    /**
     * Switch エラー
     */

    // Switch エラー構造体
    #[derive(Debug)]
    pub struct SwichError {
        state: StateBoxType,
        message: String,
    }

    // Switch エラー実装
    impl SendFsmError for SwichError {}

    impl fmt::Display for SwichError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "state: {:?}, message: {}", &self.state, &self.message)
        }
    }

    impl error::Error for SwichError {}

    /**
     * Switch コンテキスト
     */

    // Switch コンテキスト構造体
    #[derive(Debug, Clone)]
    pub struct SwichContext {
        pub is_running: bool,
    }

    // Switch コンテキスト実装
    impl SwichContext {
        pub fn new() -> SwichContext {
            SwichContext { is_running: false }
        }
    }

    /**
     * Switch イベント
     */
    #[allow(nonstandard_style)]
    #[derive(Debug, Clone)]
    pub enum SwichEvent {
        START,
        STOP,
        INTERNAL(String),
        IGNORE,
    }

    /**
     * Switch ステート構造体
     */
    #[allow(nonstandard_style)]
    pub mod SwichState {

        // RUNNING ステート
        #[derive(Debug)]
        pub struct RUNNING;

        // STOP ステート
        #[derive(Debug)]
        pub struct STOPED;
    }

    // RUNNING Switch ステート実装
    impl FsmState for SwichState::RUNNING {
        type Event = EventType;
        type Ctx = CtxType;
        type Error = Box<ErrorType>;

        fn as_box(
            &self,
        ) -> Box<dyn FsmState<Event = Self::Event, Ctx = Self::Ctx, Error = Self::Error>> {
            Box::new(Self)
        }

        fn to_transition(
            &self,
            event: &Self::Event,
        ) -> Result<FsmTransition<Self::Event, Self::Ctx, Self::Error>, Self::Error> {
            match event {
                SwichEvent::STOP => Ok(FsmTransition::External(SwichState::STOPED.as_box())),
                SwichEvent::INTERNAL(_) => Ok(FsmTransition::Internal),
                SwichEvent::IGNORE => Ok(FsmTransition::Ignore),
                _ => Err(Box::new(EventNotFoundFsmError::new(event.clone()))),
            }
        }

        fn on_entry(&self, event: &Self::Event, ctx: Self::Ctx) -> Result<Self::Ctx, Self::Error> {
            eprintln!("on_entory state:{:?} event:{:?}", self, event);
            let ctx = Arc::clone(&ctx);
            ctx.write().unwrap().is_running = true;
            Ok(ctx)
        }

        fn on_do(&self, event: &Self::Event, ctx: Self::Ctx) -> Result<Self::Ctx, Self::Error> {
            eprintln!("on_do state:{:?} event:{:?}", self, event);
            if let &SwichEvent::INTERNAL(v) = &event {
                eprintln!("internal: `{}`", v);
            }
            Ok(ctx)
        }
        fn on_exit(&self, event: &Self::Event, ctx: Self::Ctx) -> Result<Self::Ctx, Self::Error> {
            eprintln!("on_exit state:{:?} event:{:?}", self, event);
            Ok(ctx)
        }
    }

    // STOP Switch ステート実装
    impl FsmState for SwichState::STOPED {
        type Event = EventType;
        type Ctx = CtxType;
        type Error = Box<ErrorType>;

        fn as_box(
            &self,
        ) -> Box<dyn FsmState<Event = Self::Event, Ctx = Self::Ctx, Error = Self::Error>> {
            Box::new(Self)
        }

        fn to_transition(
            &self,
            event: &Self::Event,
        ) -> Result<FsmTransition<Self::Event, Self::Ctx, Self::Error>, Self::Error> {
            match &event {
                SwichEvent::START => Ok(FsmTransition::External(SwichState::RUNNING.as_box())),
                SwichEvent::INTERNAL(_) => Ok(FsmTransition::Internal),
                SwichEvent::IGNORE => Ok(FsmTransition::Ignore),
                _ => Err(Box::new(EventNotFoundFsmError::new(event.clone()))),
            }
        }

        fn on_entry(&self, event: &Self::Event, ctx: Self::Ctx) -> Result<Self::Ctx, Self::Error> {
            eprintln!("on_entory state:{:?} event:{:?}", self, event);
            let ctx = ctx;
            ctx.write().unwrap().is_running = false;
            Ok(ctx)
        }

        fn on_do(&self, event: &Self::Event, ctx: Self::Ctx) -> Result<Self::Ctx, Self::Error> {
            eprintln!("on_do state:{:?} event:{:?}", self, event);
            match &event {
                &SwichEvent::INTERNAL(v) if v == "ERROR" => {
                    let err = SwichError {
                        state: self.as_box(),
                        message: "ERROR".to_string(),
                    };
                    return Err(Box::new(err));
                }
                &SwichEvent::INTERNAL(v) => {
                    eprintln!("internal: `{}`", v);
                }
                _ => {}
            }
            Ok(ctx)
        }

        fn on_exit(&self, event: &Self::Event, ctx: Self::Ctx) -> Result<Self::Ctx, Self::Error> {
            eprintln!("on_exit state:{:?} event:{:?}", self, event);
            Ok(ctx)
        }
    }

    #[test]
    fn swich() {
        let fire = |s: StateBoxType, c: Arc<RwLock<SwichContext>>, e: SwichEvent| {
            let clone = Arc::clone(&c);
            move || s.fire(e, clone)
        };
        let ctx = SwichContext::new();
        let ctx = RwLock::new(ctx);
        let ctx = Arc::new(ctx);

        //SwichState::STOPED
        let state = SwichState::STOPED.as_box();
        assert_eq!(&ctx.read().unwrap().is_running, &false);
        assert_eq!(format!("{:?}", &state), "STOPED".to_string());

        //SwichState::RUNNING
        let join_handle = thread::spawn(fire(state, ctx, SwichEvent::START));
        let (state, ctx) = join_handle.join().unwrap().unwrap();
        assert_eq!(format!("{:?}", &state), "RUNNING".to_string());
        assert_eq!(&ctx.read().unwrap().is_running, &true);

        //SwichState::RUNNING
        let join_handle = thread::spawn(fire(
            state,
            ctx,
            SwichEvent::INTERNAL("hello world.".to_string()),
        ));
        let (state, ctx) = join_handle.join().unwrap().unwrap();
        assert_eq!(&ctx.read().unwrap().is_running, &true);

        //SwichState::RUNNING
        let join_handle = thread::spawn(fire(state, ctx, SwichEvent::IGNORE));
        let (state, ctx) = join_handle.join().unwrap().unwrap();
        assert_eq!(&ctx.read().unwrap().is_running, &true);

        //SwichState::STOPED
        let join_handle = thread::spawn(fire(state, ctx, SwichEvent::STOP));
        let result = join_handle.join().unwrap();
        assert_eq!(result.is_ok(), true);

        //SwichState::STOPED
        let (state, ctx) = result.unwrap();
        let join_handle = thread::spawn(fire(state, ctx, SwichEvent::STOP));
        let result = join_handle.join().unwrap();
        match result {
            Err(v) => assert_eq!(v.to_string(), "event not found: STOP".to_string()),
            Ok((v, _)) => {
                panic!("state: {:?}, event: {:?}", v, SwichEvent::STOP)
            }
        }
    }

    #[test]
    fn error() {
        let fire = |s: StateBoxType, c: Arc<RwLock<SwichContext>>, e: SwichEvent| {
            let clone = Arc::clone(&c);
            move || s.fire(e, clone)
        };
        let ctx = SwichContext::new();
        let ctx = RwLock::new(ctx);
        let ctx = Arc::new(ctx);

        let state = SwichState::STOPED.as_box();
        let event = SwichEvent::INTERNAL("ERROR".to_string());
        let join_handle = thread::spawn(fire(state, ctx, event.clone()));
        let result = join_handle.join().unwrap();
        match result {
            Ok(v) => panic!("state: {:?}, event: {:?}", v, &event),
            Err(v) => assert_eq!(v.to_string(), "state: STOPED, message: ERROR"),
        }
    }
}
