pub mod channel;
pub mod serialize;
pub mod predicate;
pub mod effect;

pub use rumpsteak_macros::{session, Message, Role, Roles};
use predicate::Predicate;
use effect::SideEffect;

use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use std::{
    any::Any,
    convert::Infallible,
    future::Future,
    marker::{self, PhantomData},
    fmt::Debug,
};
use thiserror::Error;
use std::collections::HashMap;

pub type SendError<Q, R> = <<Q as Route<R>>::Route as Sink<<Q as Role>::Message>>::Error;

#[derive(Debug, Error)]
pub enum ReceiveError {
    #[error("receiver stream is empty")]
    EmptyStream,
    #[error("received message with an unexpected type")]
    UnexpectedType,
}

/// This trait represents a message to be exchanged between two participants.
/// The generic type L is the type of the label (i.e. the content of the
/// message).
pub trait Message<L>: Sized {
    /// Creates a message from a label.
    fn upcast(label: L) -> Self;

    /// Tries to get the label contained in the `Message`. This might fail,
    /// typically if we are trying to get a label of the wrong type. In case of
    /// failure, the result contains `self`, hence the message is not lost.
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

/// This structure is mainly a placeholder for a `Role` and for types.
/// Typically, each each state (in the sense of automata state) of the protocol,
/// e.g. a `Send`, a `Receive`, etc, contains a `State`, as well as some type
/// bounds. When an action is taken (e.g. when `send` is called on a `Send`),
/// the `Send` will take it state and convert it into the continuation.
///
/// The generic `N` is the type of variable names and `V` the type of variable values.
pub struct State<'r, R: Role, N, V>
{
    role: &'r mut R,
    variables: HashMap<N, V>,
}

impl<'r, R: Role, N, V> State<'r, R, N, V> 
{
    #[inline]
    fn new(role: &'r mut R, variables: HashMap<N, V>) -> Self 
    {
        Self {
            role,
            variables,
        }
    }
}

pub trait FromState<'r> {
    type Role: Role;
    type Name;
    type Value;

    fn from_state(state: State<'r, Self::Role, Self::Name, Self::Value>) -> Self;
}

pub trait Session<'r>: FromState<'r> + private::Session {}

pub trait IntoSession<'r>: FromState<'r> {
    type Session: Session<'r, Role = Self::Role>;

    fn into_session(self) -> Self::Session;
}

/// This structure represents a terminated protocol.
pub struct End<'r, R: Role, N, V> 
{
    _p: PhantomData<(R, N, V, &'r ())>,
}

impl<'r, R: Role, N, V> FromState<'r> for End<'r, R, N, V> 
{
    type Role = R;
    type Name = N;
    type Value = V;

    #[inline]
    fn from_state(_state: State<'r, Self::Role, N, V>) -> Self {
        Self { _p: PhantomData }
    }
}

impl<'r, R: Role, N, V> private::Session for End<'r, R, N, V> 
{}

impl<'r, R: Role, N, V> Session<'r> for End<'r, R, N, V> 
{}

/// This structure represents a protocol which next action is to send.
pub struct Send<'q, Q: Role, N, V, R, L, P, U, S: FromState<'q, Role = Q, Name = N, Value = V>> 
where
    P: Predicate<Name = N, Value = V, Label = L>,
    U: SideEffect<Name = N, Value = V>,
{
    state: State<'q, Q, N, V>,
    phantom: PhantomData<(R, L, S)>,
    predicate: P,
    effect: U
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q, Name = N, Value = V>, N, V, P, U> FromState<'q> for Send<'q, Q, N, V, R, L, P, U, S>
where
    P: Predicate<Name = N, Value = V, Label = L>,
    U: SideEffect<Name = N, Value = V>,
{
    type Role = Q;
    type Name = N;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, N, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
            predicate: P::default(),
            effect: U::default(),
        }
    }
}

impl<'q, Q: Route<R>, R, L, S, N, V, P, U> Send<'q, Q, N, V, R, L, P, U, S>
where
    Q::Message: Message<L>,
    Q::Route: Sink<Q::Message> + Unpin,
    S: FromState<'q, Role = Q, Name = N, Value = V>,
    P: Predicate<Name = N, Value = V, Label = L>,
    U: SideEffect<Name = N, Value = V>,
{
    #[inline]
    pub async fn send(mut self, label: L) -> Result<S, SendError<Q, R>> {
        self.predicate.check(&self.state.variables, Some(&label)).unwrap();
        self.state.role.route().send(Message::upcast(label)).await?;
        self.effect.side_effect(&mut self.state.variables);
        Ok(FromState::from_state(self.state))
    }
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q, Name = N, Value = V>, N, V, P, U> private::Session for Send<'q, Q, N, V, R, L, P, U, S> 
where
    P: Predicate<Name = N, Value = V, Label = L>,
    U: SideEffect<Name = N, Value = V>,
{}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q, Name = N, Value = V>, N, V, P, U> Session<'q> for Send<'q, Q, N, V, R, L, P, U, S> 
where
    P: Predicate<Name = N, Value = V, Label = L>,
    U: SideEffect<Name = N, Value = V>,
{}

/// This structure represents a protocol which next action is to receive.
pub struct Receive<'q, Q: Role, N, V, R, L, P, U, S: FromState<'q, Role = Q>> 
where
    P: Predicate<Name = N, Value = V, Label = L>,
    U: SideEffect<Name = N, Value = V>,
{
    state: State<'q, Q, N, V>,
    phantom: PhantomData<(R, L, S)>,
    predicate: P,
    effect: U
}

impl<'q, Q: Role, R, L, S, N, V, P, U> FromState<'q> for Receive<'q, Q, N, V, R, L, P, U, S> 
where
    P: Predicate<Name = N, Value = V, Label = L>,
    U: SideEffect<Name = N, Value = V>,
    S: FromState<'q, Role = Q>,
{
    type Role = Q;
    type Name = N;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, N, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
            predicate: P::default(),
            effect: U::default(),
        }
    }
}

impl<'q, Q: Route<R>, R, L, S, N, V, P, U> Receive<'q, Q, N, V, R, L, P, U, S>
where
    Q::Message: Message<L>,
    Q::Route: Stream<Item = Q::Message> + Unpin,
    P: Predicate<Name = N, Value = V, Label = L>,
    U: SideEffect<Name = N, Value = V>,
    S: FromState<'q, Role = Q, Name = N, Value = V>,
{
    #[inline]
    pub async fn receive(mut self) -> Result<(L, S), ReceiveError> {
        self.predicate.check(&self.state.variables, None).unwrap();
        let message = self.state.role.route().next().await;
        let message = message.ok_or(ReceiveError::EmptyStream)?;
        let label = message.downcast().or(Err(ReceiveError::UnexpectedType))?;
        self.effect.side_effect(&mut self.state.variables);
        Ok((label, FromState::from_state(self.state)))
    }
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, N, V, P, U> private::Session for Receive<'q, Q, N, V, R, L, P, U, S> 
where
    P: Predicate<Name = N, Value = V, Label = L>,
    U: SideEffect<Name = N, Value = V>,
{}

impl<'q, Q: Role, R, L, S, N, V, P, U> Session<'q> for Receive<'q, Q, N, V, R, L, P, U, S> 
where
    P: Predicate<Name = N, Value = V, Label = L>,
    U: SideEffect<Name = N, Value = V>,
    S: FromState<'q, Role = Q>,
{}

pub trait Choice<'r, L> {
    type Session: FromState<'r>;
}

pub struct Select<'q, Q: Role, N, V, R, P, S, C> 
where
    P: Predicate<Name = N, Value = V>,
    S: SideEffect<Name = N, Value = V>
{
    state: State<'q, Q, N, V>,
    phantom: PhantomData<(R, C)>,
    predicate: P,
    effect: S,
}

impl<'q, Q: Role, R, C, N, V, P, S> FromState<'q> for Select<'q, Q, N, V, R, P, S, C> 
where
    P: Predicate<Name = N, Value = V>,
    S: SideEffect<Name = N, Value = V>
{
    type Role = Q;
    type Name = N;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, N, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
            predicate: P::default(),
            effect: S::default(),
        }
    }
}

impl<'q, Q: Route<R>, R, C, N, V, P, S, L> Select<'q, Q, N, V, R, P, S, C>
where
    Q::Route: Sink<Q::Message> + Unpin,
    P: Predicate<Name = N, Value = V, Label = L>,
    S: SideEffect<Name = N, Value = V>
{
    #[inline]
    pub async fn select(mut self, label: L) -> Result<<C as Choice<'q, L>>::Session, SendError<Q, R>>
    where
        Q::Message: Message<L>,
        C: Choice<'q, L>,
        C::Session: FromState<'q, Role = Q, Name = N, Value = V>,
    {
        self.predicate.check(&self.state.variables, Some(&label)).unwrap();
        self.state.role.route().send(Message::upcast(label)).await?;
        self.effect.side_effect(&mut self.state.variables);
        Ok(FromState::from_state(self.state))
    }
}

impl<'q, Q: Role, R, C, N, V, P, S> private::Session for Select<'q, Q, N, V, R, P, S, C> 
where
    P: Predicate<Name = N, Value = V>,
    S: SideEffect<Name = N, Value = V>
{}

impl<'q, Q: Role, R, C, N, V, P, S> Session<'q> for Select<'q, Q, N, V, R, P, S, C> 
where
    P: Predicate<Name = N, Value = V>,
    S: SideEffect<Name = N, Value = V>
{}

pub trait Choices<'r>: Sized {
    type Role: Role;
    type Name;
    type Value;

    fn downcast(
        state: State<'r, Self::Role, Self::Name, Self::Value>,
        message: <Self::Role as Role>::Message,
    ) -> Result<Self, <Self::Role as Role>::Message>;
}

pub struct Branch<'q, Q: Role, N, V, R, P, S, C> 
where
    P: Predicate<Name = N, Value = V>,
    S: SideEffect<Name = N, Value = V>,
{
    state: State<'q, Q, N, V>,
    phantom: PhantomData<(R, C)>,
    predicate: P,
    effect: S,
}

impl<'q, Q: Role, R, C, N, V, P, S> FromState<'q> for Branch<'q, Q, N, V, R, P, S, C> 
where
    P: Predicate<Name = N, Value = V>,
    S: SideEffect<Name = N, Value = V>,
{
    type Role = Q;
    type Name = N;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, N, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
            predicate: P::default(),
            effect: S::default(),
        }
    }
}

impl<'q, Q: Route<R>, R, C, N, V, P, S> Branch<'q, Q, N, V, R, P, S, C>
where
    Q::Route: Stream<Item = Q::Message> + Unpin,
    P: Predicate<Name = N, Value = V>,
    S: SideEffect<Name = N, Value = V>,
    C: Choices<'q, Role = Q, Name = N, Value = V>,
{
    #[inline]
    pub async fn branch(mut self) -> Result<C, ReceiveError> {
        self.predicate.check(&self.state.variables, None).unwrap();
        let message = self.state.role.route().next().await;
        let message = message.ok_or(ReceiveError::EmptyStream)?;
        self.effect.side_effect(&mut self.state.variables);
        let choice = C::downcast(self.state, message);
        choice.or(Err(ReceiveError::UnexpectedType))
    }
}

impl<'q, Q: Role, R, C, N, V, P, S> private::Session for Branch<'q, Q, N, V, R, P, S, C> 
where
    P: Predicate<Name = N, Value = V>,
    S: SideEffect<Name = N, Value = V>,
{}

impl<'q, Q: Role, R, C, N, V, P, S> Session<'q> for Branch<'q, Q, N, V, R, P, S, C> 
where
    P: Predicate<Name = N, Value = V>,
    S: SideEffect<Name = N, Value = V>,
{}

#[inline]
pub async fn session<'r, R: Role, S, T, F, N, V, P, U, L>(
    role: &'r mut R,
    map: HashMap<N, V>,
    f: impl FnOnce(S) -> F,
) -> T
where
    F: Future<Output = (T, End<'r, R, N, V>)>,
    P: Predicate<Name = N, Value = V>,
    U: SideEffect<Name = N, Value = V>,
    S: FromState<'r, Role = R, Name = N, Value = V>
{
    let output = try_session(role, map, |s| f(s).map(Ok)).await;
    output.unwrap_or_else(|infallible: Infallible| match infallible {})
}

#[inline]
pub async fn try_session<'r, R: Role, S, T, E, F, N, V>(
    role: &'r mut R,
    map: HashMap<N, V>,
    f: impl FnOnce(S) -> F,
) -> Result<T, E>
where
    F: Future<Output = Result<(T, End<'r, R, N, V>), E>>,
    S: FromState<'r, Role = R, Name = N, Value = V>
{
    let session = FromState::from_state(State::new(role, map));
    f(session).await.map(|(output, _)| output)
}

mod private {
    pub trait Session {}
}
