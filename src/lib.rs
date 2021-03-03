pub mod channel;

pub use rumpsteak_macros::{Choice, IntoSession, Message, Role, Roles};

use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use std::{any::Any, convert::Infallible, future::Future, marker::PhantomData};
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

pub trait Role {
    type Message;
}

pub trait Route<R>: Role + Sized {
    type Route;

    fn route(&mut self) -> &mut Self::Route;
}

pub struct State {
    _private: (),
}

impl State {
    #[inline]
    fn new() -> Self {
        Self { _private: () }
    }
}

pub trait Session {
    fn from_state(state: State) -> Self;
}

pub trait IntoSession: Session {
    type Session: Session;

    fn into_session(self) -> Self::Session;
}

pub struct End {
    _state: State,
}

impl Session for End {
    #[inline]
    fn from_state(state: State) -> Self {
        Self { _state: state }
    }
}

pub struct Send<R, L, S: Session> {
    state: State,
    phantom: PhantomData<(R, L, S)>,
}

impl<R, L, S: Session> Session for Send<R, L, S> {
    #[inline]
    fn from_state(state: State) -> Self {
        let phantom = PhantomData;
        Self { state, phantom }
    }
}

impl<R, L, S: Session> Send<R, L, S> {
    #[inline]
    pub async fn send<Q: Route<R>>(self, role: &mut Q, label: L) -> Result<S, SendError<Q, R>>
    where
        Q::Message: Message<L>,
        Q::Route: Sink<Q::Message> + Unpin,
    {
        role.route().send(Message::upcast(label)).await?;
        Ok(Session::from_state(self.state))
    }
}

pub struct Receive<R, L, S: Session> {
    state: State,
    phantom: PhantomData<(R, L, S)>,
}

impl<R, L, S: Session> Session for Receive<R, L, S> {
    #[inline]
    fn from_state(state: State) -> Self {
        let phantom = PhantomData;
        Self { state, phantom }
    }
}

impl<R, L, S: Session> Receive<R, L, S> {
    #[inline]
    pub async fn receive<Q: Route<R>>(self, role: &mut Q) -> Result<(L, S), ReceiveError>
    where
        Q::Message: Message<L>,
        Q::Route: Stream<Item = Q::Message> + Unpin,
    {
        let message = role.route().next().await;
        let message = message.ok_or(ReceiveError::EmptyStream)?;
        let label = message.downcast().or(Err(ReceiveError::UnexpectedType))?;
        Ok((label, Session::from_state(self.state)))
    }
}

pub trait Choice<L> {
    type Session: Session;
}

pub struct Select<R, C> {
    state: State,
    phantom: PhantomData<(R, C)>,
}

impl<R, C> Session for Select<R, C> {
    #[inline]
    fn from_state(state: State) -> Self {
        let phantom = PhantomData;
        Self { state, phantom }
    }
}

impl<R, C> Select<R, C> {
    #[inline]
    pub async fn select<Q: Route<R>, L>(
        self,
        role: &mut Q,
        label: L,
    ) -> Result<C::Session, SendError<Q, R>>
    where
        Q::Message: Message<L>,
        Q::Route: Sink<Q::Message> + Unpin,
        C: Choice<L>,
    {
        role.route().send(Message::upcast(label)).await?;
        Ok(Session::from_state(self.state))
    }
}

pub trait Choices<M>: Sized {
    fn downcast(state: State, message: M) -> Result<Self, M>;
}

pub struct Branch<R, C> {
    state: State,
    phantom: PhantomData<(R, C)>,
}

impl<R, C> Session for Branch<R, C> {
    #[inline]
    fn from_state(state: State) -> Self {
        let phantom = PhantomData;
        Self { state, phantom }
    }
}

impl<R, C> Branch<R, C> {
    #[inline]
    pub async fn branch<Q: Route<R>>(self, role: &mut Q) -> Result<C, ReceiveError>
    where
        Q::Route: Stream<Item = Q::Message> + Unpin,
        C: Choices<Q::Message>,
    {
        let message = role.route().next().await;
        let message = message.ok_or(ReceiveError::EmptyStream)?;
        C::downcast(self.state, message).or(Err(ReceiveError::UnexpectedType))
    }
}

#[inline]
pub async fn session<'r, S: Session, T, F>(f: impl FnOnce(S) -> F) -> T
where
    F: Future<Output = (T, End)>,
{
    let output = try_session(|s| f(s).map(Ok)).await;
    output.unwrap_or_else(|infallible: Infallible| match infallible {})
}

#[inline]
pub async fn try_session<'r, S: Session, T, E, F>(f: impl FnOnce(S) -> F) -> Result<T, E>
where
    F: Future<Output = Result<(T, End), E>>,
{
    let session = Session::from_state(State::new());
    f(session).await.map(|(output, _)| output)
}
