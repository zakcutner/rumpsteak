pub mod channel;
pub mod effect;
pub mod predicate;
pub mod serialize;

use effect::SideEffect;
use predicate::Predicate;
pub use rumpsteak_macros::{session, Message, Role, Roles};

use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt};
use std::collections::HashMap;
use std::{
    any::Any,
    convert::Infallible,
    fmt::Debug,
    future::Future,
    marker::{self, PhantomData},
};
use thiserror::Error;

pub type SendError<Q, R> = <<Q as Route<R>>::Route as Sink<<Q as Role>::Message>>::Error;

// The type for variable' names.
type Name = char;

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
pub struct State<'r, R: Role, V> {
    role: &'r mut R,
    variables: HashMap<Name, V>,
}

impl<'r, R: Role, V> State<'r, R, V> {
    #[inline]
    fn new(role: &'r mut R, variables: HashMap<Name, V>) -> Self {
        Self { role, variables }
    }
}

pub trait FromState<'r> {
    type Role: Role;
    type Value;

    fn from_state(state: State<'r, Self::Role, Self::Value>) -> Self;
}

pub trait Session<'r>: FromState<'r> + private::Session {}

pub trait IntoSession<'r>: FromState<'r> {
    type Session: Session<'r, Role = Self::Role>;

    fn into_session(self) -> Self::Session;
}

/// This structure represents a terminated protocol.
pub struct End<'r, R: Role, V> {
    _p: PhantomData<(R, V, &'r ())>,
}

impl<'r, R: Role, V> FromState<'r> for End<'r, R, V> {
    type Role = R;
    type Value = V;

    #[inline]
    fn from_state(_state: State<'r, Self::Role, V>) -> Self {
        Self { _p: PhantomData }
    }
}

impl<'r, R: Role, V> private::Session for End<'r, R, V> {}

impl<'r, R: Role, V> Session<'r> for End<'r, R, V> {}

/// This structure represents a protocol which next action is to send.
pub struct Send<'q, Q: Role, V, R, const NAME: Name, L, P, U, S: FromState<'q, Role = Q, Value = V>>
where
    P: Predicate<Name = Name, Value = V>,
    U: SideEffect<Name = Name, Value = V>,
    L: Clone,
    V: From<L>,
{
    state: State<'q, Q, V>,
    phantom: PhantomData<(R, L, S)>,
    predicate: P,
    effect: U,
}

impl<'q, Q: Role, R, const NAME: Name, L, S: FromState<'q, Role = Q, Value = V>, V, P, U> FromState<'q>
    for Send<'q, Q, V, R, NAME, L, P, U, S>
where
    P: Predicate<Name = Name, Value = V>,
    U: SideEffect<Name = Name, Value = V>,
    V: From<L>,
    L: Clone
{
    type Role = Q;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
            predicate: P::default(),
            effect: U::default(),
        }
    }
}

impl<'q, Q: Route<R>, R, const NAME: Name, L, S, V, P, U> Send<'q, Q, V, R, NAME, L, P, U, S>
where
    Q::Message: Message<L>,
    Q::Route: Sink<Q::Message> + Unpin,
    S: FromState<'q, Role = Q, Value = V>,
    P: Predicate<Name = Name, Value = V, Label = L>,
    U: SideEffect<Name = Name, Value = V>,
    V: From<L>,
    L: Clone,
{
    #[inline]
    pub async fn send(mut self, label: L) -> Result<S, SendError<Q, R>> {
        // TODO
        self.state.variables.insert(NAME, label.clone().into());
        self.predicate
            .check(&self.state.variables, Some(&label))
            .unwrap();
        self.state.role.route().send(Message::upcast(label)).await?;
        self.effect.side_effect(&mut self.state.variables);
        self.state.variables.remove(&NAME);
        Ok(FromState::from_state(self.state))
    }
}

impl<'q, Q: Role, R, const NAME: Name, L, S: FromState<'q, Role = Q, Value = V>, V, P, U>
    private::Session for Send<'q, Q, V, R, NAME, L, P, U, S>
where
    P: Predicate<Name = Name, Value = V, Label = L>,
    U: SideEffect<Name = Name, Value = V>,
    V: From<L>,
    L: Clone,
{
}

impl<'q, Q: Role, R, const NAME: Name, L, S: FromState<'q, Role = Q, Value = V>, V, P, U> Session<'q>
    for Send<'q, Q, V, R, NAME, L, P, U, S>
where
    P: Predicate<Name = Name, Value = V, Label = L>,
    U: SideEffect<Name = Name, Value = V>,
    V: From<L>,
    L: Clone
{
}

/// This structure represents a protocol which next action is to receive.
pub struct Receive<'q, Q: Role, V, R, const NAME: Name, L, P, U, S: FromState<'q, Role = Q>>
where
    P: Predicate<Name = Name, Value = V>,
    U: SideEffect<Name = Name, Value = V>,
{
    state: State<'q, Q, V>,
    phantom: PhantomData<(R, L, S)>,
    predicate: P,
    effect: U,
}

impl<'q, Q: Role, R, const NAME: Name, L, S, V, P, U> FromState<'q> for Receive<'q, Q, V, R, NAME, L, P, U, S>
where
    P: Predicate<Name = Name, Value = V>,
    U: SideEffect<Name = Name, Value = V>,
    S: FromState<'q, Role = Q>,
{
    type Role = Q;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
            predicate: P::default(),
            effect: U::default(),
        }
    }
}

impl<'q, Q: Route<R>, R, const NAME: Name, L, S, V, P, U> Receive<'q, Q, V, R, NAME, L, P, U, S>
where
    Q::Message: Message<L>,
    Q::Route: Stream<Item = Q::Message> + Unpin,
    P: Predicate<Name = Name, Value = V, Label = L>,
    U: SideEffect<Name = Name, Value = V>,
    S: FromState<'q, Role = Q, Value = V>,
    L: Clone,
    V: From<L>,
{
    #[inline]
    pub async fn receive(mut self) -> Result<(L, S), ReceiveError> {
        let message = self.state.role.route().next().await;
        let message = message.ok_or(ReceiveError::EmptyStream)?;
        let label = message.downcast().or(Err(ReceiveError::UnexpectedType))?;
        self.state.variables.insert(NAME, label.clone().into());
        self.effect.side_effect(&mut self.state.variables);
        //self.predicate.check(&self.state.variables, None).unwrap();
        Ok((label, FromState::from_state(self.state)))
    }
}

impl<'q, Q: Role, R, const NAME: Name, L, S: FromState<'q, Role = Q>, V, P, U> private::Session
    for Receive<'q, Q, V, R, NAME, L, P, U, S>
where
    P: Predicate<Name = Name, Value = V, Label = L>,
    U: SideEffect<Name = Name, Value = V>,
{
}

impl<'q, Q: Role, R, const NAME: Name, L, S, V, P, U> Session<'q> for Receive<'q, Q, V, R, NAME, L, P, U, S>
where
    P: Predicate<Name = Name, Value = V, Label = L>,
    U: SideEffect<Name = Name, Value = V>,
    S: FromState<'q, Role = Q>,
{
}

pub trait Choice<'r, L> {
    type Session: FromState<'r>;
}

/// This trait indicates that we can get a param name from the structure that implement them.
/// Typically for choices (in which case the parameter can be different depending on the selected
/// possibility of the choice).
pub trait ParamName<L, N> {
    fn get_param_name() -> N;
}

pub trait Param<N, V, M> {
    fn get_param(message: &M) -> (N, V);
}

pub struct Select<'q, Q: Role, V, R, P, S, C>
where
    P: Predicate<Name = Name, Value = V>,
    S: SideEffect<Name = Name, Value = V>,
{
    state: State<'q, Q, V>,
    phantom: PhantomData<(R, C)>,
    predicate: P,
    effect: S,
}

impl<'q, Q: Role, R, C, V, P, S> FromState<'q> for Select<'q, Q, V, R, P, S, C>
where
    P: Predicate<Name = Name, Value = V>,
    S: SideEffect<Name = Name, Value = V>,
{
    type Role = Q;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
            predicate: P::default(),
            effect: S::default(),
        }
    }
}

impl<'q, Q: Route<R>, R, C, V, P, S> Select<'q, Q, V, R, P, S, C>
where
    Q::Route: Sink<Q::Message> + Unpin,
    P: Predicate<Name = Name, Value = V, Label = Q::Message>,
    S: SideEffect<Name = Name, Value = V>,
{
    #[inline]
    pub async fn select<L>(
        mut self,
        label: L,
    ) -> Result<<C as Choice<'q, L>>::Session, SendError<Q, R>>
    where
        Q::Message: Message<L>,
        C: Choice<'q, L>,
        C::Session: FromState<'q, Role = Q, Value = V>,
        C: ParamName<L, Name>,
        L: Clone,
        V: From<L>,
    {
        let param_name = C::get_param_name();
        self.state.variables.insert(param_name, label.clone().into());
        let msg = Message::upcast(label);
        self.predicate
            .check(&self.state.variables, Some(&msg))
            .unwrap();
        self.state.role.route().send(msg).await?;
        self.effect.side_effect(&mut self.state.variables);
        self.state.variables.remove(&param_name);
        Ok(FromState::from_state(self.state))
    }
}

impl<'q, Q: Role, R, C, V, P, S> private::Session for Select<'q, Q, V, R, P, S, C>
where
    P: Predicate<Name = Name, Value = V>,
    S: SideEffect<Name = Name, Value = V>,
{
}

impl<'q, Q: Role, R, C, V, P, S> Session<'q> for Select<'q, Q, V, R, P, S, C>
where
    P: Predicate<Name = Name, Value = V>,
    S: SideEffect<Name = Name, Value = V>,
{
}

pub trait Choices<'r>: Sized {
    type Role: Role;
    type Value;

    fn downcast(
        state: State<'r, Self::Role, Self::Value>,
        message: <Self::Role as Role>::Message,
    ) -> Result<Self, <Self::Role as Role>::Message>;
}

pub struct Branch<'q, Q: Role, V, R, P, S, C>
where
    P: Predicate<Name = Name, Value = V>,
    S: SideEffect<Name = Name, Value = V>,
{
    state: State<'q, Q, V>,
    phantom: PhantomData<(R, C)>,
    predicate: P,
    effect: S,
}

impl<'q, Q: Role, R, C, V, P, S> FromState<'q> for Branch<'q, Q, V, R, P, S, C>
where
    P: Predicate<Name = Name, Value = V>,
    S: SideEffect<Name = Name, Value = V>,
{
    type Role = Q;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
            predicate: P::default(),
            effect: S::default(),
        }
    }
}

impl<'q, Q: Route<R>, R, C, V, P, S> Branch<'q, Q, V, R, P, S, C>
where
    Q::Route: Stream<Item = Q::Message> + Unpin,
    P: Predicate<Name = Name, Value = V>,
    S: SideEffect<Name = Name, Value = V>,
    C: Choices<'q, Role = Q, Value = V>,
    C: Param<Name, V, Q::Message>,
{
    #[inline]
    pub async fn branch(mut self) -> Result<C, ReceiveError> {
        let message = self.state.role.route().next().await;
        let message = message.ok_or(ReceiveError::EmptyStream)?;
        let (param_name, param_val) = C::get_param(&message);
        self.state.variables.insert(param_name, param_val);
        self.effect.side_effect(&mut self.state.variables);
        let choice = C::downcast(self.state, message);
        match choice {
            Ok(c) => {
                Ok(c)
            }
            Err(_) => {
                Err(ReceiveError::UnexpectedType)
            }
        }
    }
}

impl<'q, Q: Role, R, C, V, P, S> private::Session for Branch<'q, Q, V, R, P, S, C>
where
    P: Predicate<Name = Name, Value = V>,
    S: SideEffect<Name = Name, Value = V>,
{
}

impl<'q, Q: Role, R, C, V, P, S> Session<'q> for Branch<'q, Q, V, R, P, S, C>
where
    P: Predicate<Name = Name, Value = V>,
    S: SideEffect<Name = Name, Value = V>,
{
}

#[inline]
pub async fn session<'r, R: Role, S, T, F, V, P, U, L>(
    role: &'r mut R,
    map: HashMap<Name, V>,
    f: impl FnOnce(S) -> F,
) -> T
where
    F: Future<Output = (T, End<'r, R, V>)>,
    P: Predicate<Name = Name, Value = V>,
    U: SideEffect<Name = Name, Value = V>,
    S: FromState<'r, Role = R, Value = V>,
{
    let output = try_session(role, map, |s| f(s).map(Ok)).await;
    output.unwrap_or_else(|infallible: Infallible| match infallible {})
}

#[inline]
pub async fn try_session<'r, R: Role, S, T, E, F, V>(
    role: &'r mut R,
    map: HashMap<Name, V>,
    f: impl FnOnce(S) -> F,
) -> Result<T, E>
where
    F: Future<Output = Result<(T, End<'r, R, V>), E>>,
    S: FromState<'r, Role = R, Value = V>,
{
    let session = FromState::from_state(State::new(role, map));
    f(session).await.map(|(output, _)| output)
}

mod private {
    pub trait Session {}
}
