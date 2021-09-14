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
pub enum Error<E> {
    #[error("failed to check refinements")]
    Refinements,
    #[error(transparent)]
    Other(#[from] E),
}

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

pub struct State<'r, R: Role, M = ()> {
    role: &'r mut R,
    mapping: M,
}

impl<'r, R: Role, M: Default> State<'r, R, M> {
    #[inline]
    fn new(role: &'r mut R) -> Self {
        let mapping = Default::default();
        Self { role, mapping }
    }
}

pub trait FromState<'r> {
    type Role: Role;

    type Mapping;

    fn from_state(state: State<'r, Self::Role, Self::Mapping>) -> Self;
}

pub trait Session<'r>: FromState<'r> + private::Session {}

pub trait IntoSession<'r>: FromState<'r> {
    type Session: Session<'r, Role = Self::Role>;

    fn into_session(self) -> Self::Session;
}

pub trait Verify<M, L> {
    fn verify(mapping: &mut M, label: &L) -> bool;
}

pub struct Always;

impl<M, L> Verify<M, L> for Always {
    fn verify(_: &mut M, _: &L) -> bool {
        true
    }
}

pub struct End<'r, R: Role, M = ()> {
    _state: State<'r, R, M>,
}

impl<'r, R: Role, M> FromState<'r> for End<'r, R, M> {
    type Role = R;

    type Mapping = M;

    #[inline]
    fn from_state(state: State<'r, Self::Role, M>) -> Self {
        Self { _state: state }
    }
}

impl<'r, R: Role, M> private::Session for End<'r, R, M> {}

impl<'r, R: Role, M> Session<'r> for End<'r, R, M> {}

pub struct Send<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, M = (), V = Always> {
    state: State<'q, Q, M>,
    phantom: PhantomData<(R, L, S, V)>,
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, M, V> FromState<'q>
    for Send<'q, Q, R, L, S, M, V>
{
    type Role = Q;

    type Mapping = M;

    #[inline]
    fn from_state(state: State<'q, Self::Role, M>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, L, S: FromState<'q, Role = Q, Mapping = M>, M, V: Verify<M, L>>
    Send<'q, Q, R, L, S, M, V>
where
    Q::Message: Message<L>,
    Q::Route: Sink<Q::Message> + Unpin,
{
    #[inline]
    pub async fn send(mut self, label: L) -> Result<S, Error<SendError<Q, R>>> {
        if !V::verify(&mut self.state.mapping, &label) {
            return Err(Error::Refinements);
        }

        self.state.role.route().send(Message::upcast(label)).await?;
        Ok(FromState::from_state(self.state))
    }
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, M, V> private::Session
    for Send<'q, Q, R, L, S, M, V>
{
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, M, V> Session<'q>
    for Send<'q, Q, R, L, S, M, V>
{
}

pub struct Receive<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, M = (), V = Always> {
    state: State<'q, Q, M>,
    phantom: PhantomData<(R, L, S, V)>,
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, M, V> FromState<'q>
    for Receive<'q, Q, R, L, S, M, V>
{
    type Role = Q;

    type Mapping = M;

    #[inline]
    fn from_state(state: State<'q, Self::Role, M>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, L, S: FromState<'q, Role = Q, Mapping = M>, M, V: Verify<M, L>>
    Receive<'q, Q, R, L, S, M, V>
where
    Q::Message: Message<L>,
    Q::Route: Stream<Item = Q::Message> + Unpin,
{
    #[inline]
    pub async fn receive(mut self) -> Result<(L, S), Error<ReceiveError>> {
        let message = self.state.role.route().next().await;
        let message = message.ok_or(ReceiveError::EmptyStream)?;

        let label = message.downcast().or(Err(ReceiveError::UnexpectedType))?;
        if !V::verify(&mut self.state.mapping, &label) {
            return Err(Error::Refinements);
        }

        Ok((label, FromState::from_state(self.state)))
    }
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, M, V> private::Session
    for Receive<'q, Q, R, L, S, M, V>
{
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, M, V> Session<'q>
    for Receive<'q, Q, R, L, S, M, V>
{
}

pub trait Choice<'r, M, L> {
    type Session: FromState<'r>;

    type Verify: Verify<M, L>;
}

pub struct Select<'q, Q: Role, R, C, M = ()> {
    state: State<'q, Q, M>,
    phantom: PhantomData<(R, C)>,
}

impl<'q, Q: Role, R, C, M> FromState<'q> for Select<'q, Q, R, C, M> {
    type Role = Q;

    type Mapping = M;

    #[inline]
    fn from_state(state: State<'q, Self::Role, M>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, C, M> Select<'q, Q, R, C, M>
where
    Q::Route: Sink<Q::Message> + Unpin,
{
    #[inline]
    pub async fn select<L>(
        mut self,
        label: L,
    ) -> Result<<C as Choice<'q, M, L>>::Session, Error<SendError<Q, R>>>
    where
        Q::Message: Message<L>,
        C: Choice<'q, M, L>,
        C::Session: FromState<'q, Role = Q, Mapping = M>,
    {
        if !C::Verify::verify(&mut self.state.mapping, &label) {
            return Err(Error::Refinements);
        }

        self.state.role.route().send(Message::upcast(label)).await?;
        Ok(FromState::from_state(self.state))
    }
}

impl<'q, Q: Role, R, C, M> private::Session for Select<'q, Q, R, C, M> {}

impl<'q, Q: Role, R, C, M> Session<'q> for Select<'q, Q, R, C, M> {}

pub trait Choices<'r>: Sized {
    type Role: Role;

    type Mapping;

    fn downcast(
        state: State<'r, Self::Role, Self::Mapping>,
        message: <Self::Role as Role>::Message,
    ) -> Result<Self, <Self::Role as Role>::Message>;
}

pub struct Branch<'q, Q: Role, R, C, M = (), V = Always> {
    state: State<'q, Q, M>,
    phantom: PhantomData<(R, C, V)>,
}

impl<'q, Q: Role, R, C, M, V> FromState<'q> for Branch<'q, Q, R, C, M, V> {
    type Role = Q;

    type Mapping = M;

    #[inline]
    fn from_state(state: State<'q, Self::Role, M>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, C: Choices<'q, Role = Q, Mapping = M>, M, V: Verify<M, C>>
    Branch<'q, Q, R, C, M, V>
where
    Q::Route: Stream<Item = Q::Message> + Unpin,
{
    #[inline]
    pub async fn branch(mut self) -> Result<C, Error<ReceiveError>> {
        let message = self.state.role.route().next().await;
        let message = message.ok_or(ReceiveError::EmptyStream)?;

        let choice = C::downcast(self.state, message);
        let choice = choice.or(Err(ReceiveError::UnexpectedType))?;
        // if !V::verify(&mut self.state.mapping, &choice) {
        //     return Err(Error::Refinements);
        // }

        Ok(choice)
    }
}

impl<'q, Q: Role, R, C, M, V> private::Session for Branch<'q, Q, R, C, M, V> {}

impl<'q, Q: Role, R, C, M, V> Session<'q> for Branch<'q, Q, R, C, M, V> {}

#[inline]
pub async fn session<'r, R: Role, S: FromState<'r, Role = R>, T, F>(
    role: &'r mut R,
    f: impl FnOnce(S) -> F,
) -> T
where
    S::Mapping: Default,
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
    S::Mapping: Default,
    F: Future<Output = Result<(T, End<'r, R>), E>>,
{
    let session = FromState::from_state(State::new(role));
    f(session).await.map(|(output, _)| output)
}

mod private {
    pub trait Session {}
}
