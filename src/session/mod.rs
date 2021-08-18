#![cfg(feature = "session")]

pub mod channel;
pub mod serialize;

pub use rumpsteak_macros::{session, Message, Role, Roles};

use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use std::{
    any::Any,
    convert::Infallible,
    future::Future,
    marker::{self, PhantomData},
};
use thiserror::Error;

pub type SendError<Q, R> = <<Q as Route<R>>::Route as Sink<<Q as Role>::Message>>::Error;

#[derive(Debug, Error)]
pub enum ReceiveError {
    #[error("receiver stream is empty")]
    EmptyStream,
    #[error("received message with an unexpected type")]
    UnexpectedType,
}

pub trait Message<L>: Sized {
    fn upcast(label: L) -> Self;

    fn downcast(self) -> Result<L, Self>;
}

impl<L: 'static> Message<L> for Box<dyn Any> {
    fn upcast(label: L) -> Self {
        Box::new(label)
    }

    fn downcast(self) -> Result<L, Self> {
        self.downcast().map(|label| *label)
    }
}

impl<L: marker::Send + 'static> Message<L> for Box<dyn Any + marker::Send> {
    fn upcast(label: L) -> Self {
        Box::new(label)
    }

    fn downcast(self) -> Result<L, Self> {
        self.downcast().map(|label| *label)
    }
}

impl<L: marker::Send + Sync + 'static> Message<L> for Box<dyn Any + marker::Send + Sync> {
    fn upcast(label: L) -> Self {
        Box::new(label)
    }

    fn downcast(self) -> Result<L, Self> {
        self.downcast().map(|label| *label)
    }
}

pub trait Role {
    type Message;
}

pub trait Route<R>: Role + Sized {
    type Route;

    fn route(&mut self) -> &mut Self::Route;
}

pub struct State<'r, R: Role> {
    role: &'r mut R,
}

impl<'r, R: Role> State<'r, R> {
    #[inline]
    fn new(role: &'r mut R) -> Self {
        Self { role }
    }
}

pub trait FromState<'r> {
    type Role: Role;

    fn from_state(state: State<'r, Self::Role>) -> Self;
}

pub trait Session<'r>: FromState<'r> + private::Session {}

pub trait IntoSession<'r>: FromState<'r> {
    type Session: Session<'r, Role = Self::Role>;

    fn into_session(self) -> Self::Session;
}

pub struct End<'r, R: Role> {
    _state: State<'r, R>,
}

impl<'r, R: Role> FromState<'r> for End<'r, R> {
    type Role = R;

    #[inline]
    fn from_state(state: State<'r, Self::Role>) -> Self {
        Self { _state: state }
    }
}

impl<'r, R: Role> private::Session for End<'r, R> {}

impl<'r, R: Role> Session<'r> for End<'r, R> {}

pub struct Send<'q, Q: Role, R, L, S: FromState<'q, Role = Q>> {
    state: State<'q, Q>,
    phantom: PhantomData<(R, L, S)>,
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>> FromState<'q> for Send<'q, Q, R, L, S> {
    type Role = Q;

    #[inline]
    fn from_state(state: State<'q, Self::Role>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, L, S: FromState<'q, Role = Q>> Send<'q, Q, R, L, S>
where
    Q::Message: Message<L>,
    Q::Route: Sink<Q::Message> + Unpin,
{
    #[inline]
    pub async fn send(self, label: L) -> Result<S, SendError<Q, R>> {
        self.state.role.route().send(Message::upcast(label)).await?;
        Ok(FromState::from_state(self.state))
    }
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>> private::Session for Send<'q, Q, R, L, S> {}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>> Session<'q> for Send<'q, Q, R, L, S> {}

pub struct Receive<'q, Q: Role, R, L, S: FromState<'q, Role = Q>> {
    state: State<'q, Q>,
    phantom: PhantomData<(R, L, S)>,
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>> FromState<'q> for Receive<'q, Q, R, L, S> {
    type Role = Q;

    #[inline]
    fn from_state(state: State<'q, Self::Role>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, L, S: FromState<'q, Role = Q>> Receive<'q, Q, R, L, S>
where
    Q::Message: Message<L>,
    Q::Route: Stream<Item = Q::Message> + Unpin,
{
    #[inline]
    pub async fn receive(self) -> Result<(L, S), ReceiveError> {
        let message = self.state.role.route().next().await;
        let message = message.ok_or(ReceiveError::EmptyStream)?;
        let label = message.downcast().or(Err(ReceiveError::UnexpectedType))?;
        Ok((label, FromState::from_state(self.state)))
    }
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>> private::Session for Receive<'q, Q, R, L, S> {}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>> Session<'q> for Receive<'q, Q, R, L, S> {}

pub trait Choice<'r, L> {
    type Session: FromState<'r>;
}

pub struct Select<'q, Q: Role, R, C> {
    state: State<'q, Q>,
    phantom: PhantomData<(R, C)>,
}

impl<'q, Q: Role, R, C> FromState<'q> for Select<'q, Q, R, C> {
    type Role = Q;

    #[inline]
    fn from_state(state: State<'q, Self::Role>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, C> Select<'q, Q, R, C>
where
    Q::Route: Sink<Q::Message> + Unpin,
{
    #[inline]
    pub async fn select<L>(self, label: L) -> Result<<C as Choice<'q, L>>::Session, SendError<Q, R>>
    where
        Q::Message: Message<L>,
        C: Choice<'q, L>,
        C::Session: FromState<'q, Role = Q>,
    {
        self.state.role.route().send(Message::upcast(label)).await?;
        Ok(FromState::from_state(self.state))
    }
}

impl<'q, Q: Role, R, C> private::Session for Select<'q, Q, R, C> {}

impl<'q, Q: Role, R, C> Session<'q> for Select<'q, Q, R, C> {}

pub trait Choices<'r>: Sized {
    type Role: Role;

    fn downcast(
        state: State<'r, Self::Role>,
        message: <Self::Role as Role>::Message,
    ) -> Result<Self, <Self::Role as Role>::Message>;
}

pub struct Branch<'q, Q: Role, R, C> {
    state: State<'q, Q>,
    phantom: PhantomData<(R, C)>,
}

impl<'q, Q: Role, R, C> FromState<'q> for Branch<'q, Q, R, C> {
    type Role = Q;

    #[inline]
    fn from_state(state: State<'q, Self::Role>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, C: Choices<'q, Role = Q>> Branch<'q, Q, R, C>
where
    Q::Route: Stream<Item = Q::Message> + Unpin,
{
    #[inline]
    pub async fn branch(self) -> Result<C, ReceiveError> {
        let message = self.state.role.route().next().await;
        let message = message.ok_or(ReceiveError::EmptyStream)?;
        let choice = C::downcast(self.state, message);
        choice.or(Err(ReceiveError::UnexpectedType))
    }
}

impl<'q, Q: Role, R, C> private::Session for Branch<'q, Q, R, C> {}

impl<'q, Q: Role, R, C> Session<'q> for Branch<'q, Q, R, C> {}

#[inline]
pub async fn session<'r, R: Role, S: FromState<'r, Role = R>, T, F>(
    role: &'r mut R,
    f: impl FnOnce(S) -> F,
) -> T
where
    F: Future<Output = (T, End<'r, R>)>,
{
    let output = try_session(role, |s| f(s).map(Ok)).await;
    output.unwrap_or_else(|infallible: Infallible| match infallible {})
}

#[inline]
pub async fn try_session<'r, R: Role, S: FromState<'r, Role = R>, T, E, F>(
    role: &'r mut R,
    f: impl FnOnce(S) -> F,
) -> Result<T, E>
where
    F: Future<Output = Result<(T, End<'r, R>), E>>,
{
    let session = FromState::from_state(State::new(role));
    f(session).await.map(|(output, _)| output)
}

mod private {
    pub trait Session {}
}
