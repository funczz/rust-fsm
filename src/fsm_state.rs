use std::fmt;

use crate::FsmTransition;

pub trait FsmState
where
    Self: fmt::Debug + Send,
{
    type Event;
    type Ctx;
    type Error;

    fn as_box(
        &self,
    ) -> Box<dyn FsmState<Event = Self::Event, Ctx = Self::Ctx, Error = Self::Error>>;

    fn to_transition(
        &self,
        event: &Self::Event,
    ) -> Result<FsmTransition<Self::Event, Self::Ctx, Self::Error>, Self::Error>;

    fn on_entry(&self, event: &Self::Event, ctx: Self::Ctx) -> Result<Self::Ctx, Self::Error>;

    fn on_do(&self, event: &Self::Event, ctx: Self::Ctx) -> Result<Self::Ctx, Self::Error>;

    fn on_exit(&self, event: &Self::Event, ctx: Self::Ctx) -> Result<Self::Ctx, Self::Error>;

    fn fire(
        &self,
        event: Self::Event,
        ctx: Self::Ctx,
    ) -> Result<
        (
            Box<
                dyn FsmState<
                    Event = Self::Event,
                    Ctx = Self::Ctx,
                    Error = Self::Error,
                >,
            >,
            Self::Ctx,
        ),
        Self::Error,
    > {
        let transition = self.to_transition(&event);
        let transition = match transition {
            Ok(v) => v,
            Err(err) => return Err(err),
        };
        match transition {
            FsmTransition::Ignore => Ok((self.as_box(), ctx)),
            FsmTransition::Internal => match self.on_do(&event, ctx) {
                Ok(ctx) => Ok((self.as_box(), ctx)),
                Err(err) => return Err(err),
            },
            FsmTransition::External(state) => {
                let result = self.on_exit(&event, ctx);
                let ctx = match result {
                    Ok(ctx) => ctx,
                    Err(err) => return Err(err),
                };
                let result = state.on_entry(&event, ctx);
                let ctx = match result {
                    Ok(ctx) => ctx,
                    Err(err) => return Err(err),
                };
                let result = state.on_do(&event, ctx);
                let ctx = match result {
                    Ok(ctx) => ctx,
                    Err(err) => return Err(err),
                };
                Ok((state, ctx))
            }
        }
    }
}
