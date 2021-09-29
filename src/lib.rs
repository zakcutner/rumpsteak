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

/// A `Predicate` is a structure that allows to express some properties on some
/// variables. The trait contains a single function which assigns a truth value
/// to a set of variables.
pub trait Predicate {
    type Name;
    type Value;
    type Error;

    /// This function checks whether the predicate holds on the current values
    /// of variables.
    /// It returns a `Result` which is `Ok(())` if the predicate holds, or an
    /// arbitrary error (of type `E`) if not. 
    /// Most likely, one may want to have `()` as the error type (falling back
    /// on something simili-boolean), but having this generic type allow more
    /// precise analysis in case of failure (some kind of very basic causal
    /// analysis).
    ///
    /// The default implementation always returns `Ok(())`.
    fn check(m: &HashMap<Self::Name, Self::Value>) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// The `Tautology` struct implements a tautology predicate (i.e. always valid).
pub struct Tautology<N, V> {
    _ph: PhantomData<(N, V)>,
}

impl<N, V> Tautology<N, V> {
    fn new() -> Self {
        Self { _ph: PhantomData }
    }
}

impl<N, V> Predicate for Tautology<N, V> {
    type Name = N;
    type Value = V;
    type Error = ();
}

/// A `SideEffect` is a structure that allows arbitrary modifications of a set
/// of variables.
pub trait SideEffect {
    type Name;
    type Value;

    /// This function modifies the values of variables.
    ///
    /// The default implementation does not modify the variables.
    fn side_effect(m: &mut HashMap<Self::Name, Self::Value>) {}
}

/// The `Constant` struct implements a side effect that does nothing.
pub struct Constant<N, V> {
    _ph: PhantomData<(N, V)>,
}

impl<N, V> Constant<N, V> {
    fn new() -> Self {
        Self { _ph: PhantomData }
    }
}

impl<N, V> SideEffect for Constant<N, V> {
    type Name = N;
    type Value = V;
}

/// This structure is mainly a placeholder for a `Role` and for types.
/// Typically, each each state (in the sense of automata state) of the protocol,
/// e.g. a `Send`, a `Receive`, etc, contains a `State`, as well as some type
/// bounds. When an action is taken (e.g. when `send` is called on a `Send`),
/// the `Send` will take it state and convert it into the continuation.
///
/// The generic `N` is the type of variable names and `V` the type of variable values.
pub struct State<'r, R: Role, N = (), V = (), P = Tautology<N, V>, S = Constant<N, V>>
where
    P: Predicate<Name = N, Value = V>,
    S: SideEffect<Name = N, Value = V>,
{
    role: &'r mut R,
    variables: HashMap<N, V>,
    predicate: P,
    effect: S,
}

impl<'r, R: Role, N, V> State<'r, R, N, V> {
    #[inline]
    fn new(role: &'r mut R) -> Self {
        Self {
            role,
            variables: HashMap::new(),
            predicate: Tautology::new(),
            effect: Constant::new(),
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
pub struct End<'r, R: Role, N = (), V = ()> {
    _state: State<'r, R, N, V>,
}

impl<'r, R: Role, N, V> FromState<'r> for End<'r, R, N, V> {
    type Role = R;
    type Name = N;
    type Value = V;

    #[inline]
    fn from_state(state: State<'r, Self::Role, N, V>) -> Self {
        Self { _state: state }
    }
}

impl<'r, R: Role, N, V> private::Session for End<'r, R, N, V> {}

impl<'r, R: Role, N, V> Session<'r> for End<'r, R, N, V> {}

/// This structure represents a protocol which next action is to send.
pub struct Send<'q, Q: Role, R, L, S: FromState<'q, Role = Q, Name = N, Value = V>, N = (), V = ()> {
    state: State<'q, Q, N, V>,
    phantom: PhantomData<(R, L, S)>,
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q, Name = N, Value = V>, N, V> FromState<'q> for Send<'q, Q, R, L, S, N, V> {
    type Role = Q;
    type Name = N;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, N, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, L, S: FromState<'q, Role = Q, Name = N, Value = V>, N, V> Send<'q, Q, R, L, S, N, V>
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

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q, Name = N, Value = V>, N, V> private::Session for Send<'q, Q, R, L, S, N, V> {}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q, Name = N, Value = V>, N, V> Session<'q> for Send<'q, Q, R, L, S, N, V> {}

/// This structure represents a protocol which next action is to receive .
pub struct Receive<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, N = (), V = ()> {
    state: State<'q, Q, N, V>,
    phantom: PhantomData<(R, L, S)>,
}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, N, V> FromState<'q> for Receive<'q, Q, R, L, S, N, V> {
    type Role = Q;
    type Name = N;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, N, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, L, S: FromState<'q, Role = Q, Name = N, Value = V>, N, V> Receive<'q, Q, R, L, S, N, V>
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

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, N, V> private::Session for Receive<'q, Q, R, L, S, N, V> {}

impl<'q, Q: Role, R, L, S: FromState<'q, Role = Q>, N, V> Session<'q> for Receive<'q, Q, R, L, S, N, V> {}

pub trait Choice<'r, L> {
    type Session: FromState<'r>;
}

pub struct Select<'q, Q: Role, R, C, N = (), V = ()> {
    state: State<'q, Q, N, V>,
    phantom: PhantomData<(R, C)>,
}

impl<'q, Q: Role, R, C, N, V> FromState<'q> for Select<'q, Q, R, C, N, V> {
    type Role = Q;
    type Name = N;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, N, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, C, N, V> Select<'q, Q, R, C, N, V>
where
    Q::Route: Sink<Q::Message> + Unpin,
{
    #[inline]
    pub async fn select<L>(self, label: L) -> Result<<C as Choice<'q, L>>::Session, SendError<Q, R>>
    where
        Q::Message: Message<L>,
        C: Choice<'q, L>,
        C::Session: FromState<'q, Role = Q, Name = N, Value = V>,
    {
        self.state.role.route().send(Message::upcast(label)).await?;
        Ok(FromState::from_state(self.state))
    }
}

impl<'q, Q: Role, R, C, N, V> private::Session for Select<'q, Q, R, C, N, V> {}

impl<'q, Q: Role, R, C, N, V> Session<'q> for Select<'q, Q, R, C, N, V> {}

pub trait Choices<'r>: Sized {
    type Role: Role;
    type Name;
    type Value;

    fn downcast(
        state: State<'r, Self::Role, Self::Name, Self::Value>,
        message: <Self::Role as Role>::Message,
    ) -> Result<Self, <Self::Role as Role>::Message>;
}

pub struct Branch<'q, Q: Role, R, C, N = (), V = ()> {
    state: State<'q, Q, N, V>,
    phantom: PhantomData<(R, C)>,
}

impl<'q, Q: Role, R, C, N, V> FromState<'q> for Branch<'q, Q, R, C, N, V> {
    type Role = Q;
    type Name = N;
    type Value = V;

    #[inline]
    fn from_state(state: State<'q, Self::Role, N, V>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R, C: Choices<'q, Role = Q, Name = N, Value = V>, N, V> Branch<'q, Q, R, C, N, V>
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

impl<'q, Q: Role, R, C, N, V> private::Session for Branch<'q, Q, R, C, N, V> {}

impl<'q, Q: Role, R, C, N, V> Session<'q> for Branch<'q, Q, R, C, N, V> {}

#[inline]
pub async fn session<'r, R: Role, S: FromState<'r, Role = R>, T, F, N, V>(
    role: &'r mut R,
    f: impl FnOnce(S) -> F,
) -> T
where
    F: Future<Output = (T, End<'r, R, N, V>)>,
{
    let output = try_session(role, |s| f(s).map(Ok)).await;
    output.unwrap_or_else(|infallible: Infallible| match infallible {})
}

#[inline]
pub async fn try_session<'r, R: Role, S: FromState<'r, Role = R>, T, E, F, N, V>(
    role: &'r mut R,
    f: impl FnOnce(S) -> F,
) -> Result<T, E>
where
    F: Future<Output = Result<(T, End<'r, R, N, V>), E>>,
{
    let session = FromState::from_state(State::new(role));
    f(session).await.map(|(output, _)| output)
}

mod private {
    pub trait Session {}
}
