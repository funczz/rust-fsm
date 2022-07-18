use crate::FsmState;

#[derive(Debug)]
pub enum FsmTransition<E, C, ER> {
    Ignore,
    Internal,
    External(Box<dyn FsmState<Event = E, Ctx = C, Error = ER>>),
}
