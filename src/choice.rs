pub use session_macros::Choice;

use crate::{role::Role, Label, Session, State};

pub trait Internal<'r, R: Role, L>
where
    R::Message: Label<L>,
{
    type Session: Session<'r, R>;
}

pub trait External<'r, R: Role>: Sized {
    fn choice(state: State<'r, R>, message: R::Message) -> Option<Self>;
}
