pub mod channel;
pub mod choice;
pub mod role;

#[deprecated]
pub mod oneshot;

pub use self::channel::SendError;
pub use session_macros::{IntoSession, Label};

use self::{
    choice::{External, Internal},
    role::{Receiver, Role, Route, Sender},
};
use futures::FutureExt;
use std::{convert::Infallible, future::Future, marker::PhantomData, result};
use thiserror::Error;

pub type Result<T, E = Error> = result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Send(#[from] SendError),
    #[error(transparent)]
    Receive(#[from] ReceiveError),
}

#[derive(Debug, Error)]
pub enum ReceiveError {
    #[error(transparent)]
    Channel(#[from] channel::ReceiveError),
    #[error("received message with an unexpected type")]
    UnexpectedType,
}

pub trait Label<L> {
    fn wrap(label: L) -> Self;

    fn unwrap(self) -> Option<L>;
}

pub struct State<'r, R: Role>(&'r mut R);

impl<'r, R: Role> State<'r, R> {
    pub fn into_session<S: Session<'r, R>>(self) -> S {
        S::from_state(self)
    }
}

pub trait Session<'r, R: Role> {
    fn from_state(state: State<'r, R>) -> Self;
}

pub trait IntoSession<'r, R: Role>: Session<'r, R> {
    type Session: Session<'r, R>;

    fn into_session(self) -> Self::Session;
}

pub struct End<'r> {
    phantom: PhantomData<&'r ()>,
}

impl<'r, R: Role> Session<'r, R> for End<'r> {
    #[inline]
    fn from_state(_: State<'r, R>) -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

pub struct Send<'q, Q: Route<R>, R: Route<Q>, L, S: Session<'q, Q>>
where
    Q::Message: Label<L>,
    Q::Route: Sender<R>,
    R: Role<Message = Q::Message>,
    R::Route: Receiver<Q>,
{
    state: State<'q, Q>,
    phantom: PhantomData<(R, L, S)>,
}

impl<'q, Q: Route<R>, R: Route<Q>, L, S: Session<'q, Q>> Session<'q, Q> for Send<'q, Q, R, L, S>
where
    Q::Message: Label<L>,
    Q::Route: Sender<R>,
    R: Role<Message = Q::Message>,
    R::Route: Receiver<Q>,
{
    #[inline]
    fn from_state(state: State<'q, Q>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R: Route<Q>, L, S: Session<'q, Q>> Send<'q, Q, R, L, S>
where
    Q::Message: Label<L>,
    Q::Route: Sender<R>,
    R: Role<Message = Q::Message>,
    R::Route: Receiver<Q>,
{
    #[inline]
    pub fn send(self, label: L) -> Result<S, SendError> {
        self.state.0.route().sender().send(Label::wrap(label))?;
        Ok(self.state.into_session())
    }
}

pub struct Receive<'q, Q: Route<R>, R: Route<Q>, L, S: Session<'q, Q>>
where
    Q::Message: Label<L>,
    Q::Route: Receiver<R>,
    R: Role<Message = Q::Message>,
    R::Route: Sender<Q>,
{
    state: State<'q, Q>,
    phantom: PhantomData<(R, L, S)>,
}

impl<'q, Q: Route<R>, R: Route<Q>, L, S: Session<'q, Q>> Session<'q, Q> for Receive<'q, Q, R, L, S>
where
    Q::Message: Label<L>,
    Q::Route: Receiver<R>,
    R: Role<Message = Q::Message>,
    R::Route: Sender<Q>,
{
    #[inline]
    fn from_state(state: State<'q, Q>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R: Route<Q>, L, S: Session<'q, Q>> Receive<'q, Q, R, L, S>
where
    Q::Message: Label<L>,
    Q::Route: Receiver<R>,
    R: Role<Message = Q::Message>,
    R::Route: Sender<Q>,
{
    #[inline]
    pub async fn receive(self) -> Result<(L, S), ReceiveError> {
        let label = Label::unwrap(self.state.0.route().receiver().receive().await?);
        let label = label.ok_or(ReceiveError::UnexpectedType)?;
        Ok((label, self.state.into_session()))
    }
}

pub struct Select<'q, Q: Route<R>, R: Route<Q>, C>
where
    Q::Route: Sender<R>,
    R: Role<Message = Q::Message>,
    R::Route: Receiver<Q>,
{
    state: State<'q, Q>,
    phantom: PhantomData<(R, C)>,
}

impl<'q, Q: Route<R>, R: Route<Q>, C> Session<'q, Q> for Select<'q, Q, R, C>
where
    Q::Route: Sender<R>,
    R: Role<Message = Q::Message>,
    R::Route: Receiver<Q>,
{
    #[inline]
    fn from_state(state: State<'q, Q>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R: Route<Q>, C> Select<'q, Q, R, C>
where
    Q::Route: Sender<R>,
    R: Role<Message = Q::Message>,
    R::Route: Receiver<Q>,
{
    #[inline]
    pub fn select<L>(self, label: L) -> Result<C::Session, SendError>
    where
        Q::Message: Label<L>,
        C: Internal<'q, Q, L>,
    {
        self.state.0.route().sender().send(Label::wrap(label))?;
        Ok(self.state.into_session())
    }
}

pub struct Branch<'q, Q: Route<R>, R: Route<Q>, C: External<'q, Q>>
where
    Q::Route: Receiver<R>,
    R: Role<Message = Q::Message>,
    R::Route: Sender<Q>,
{
    state: State<'q, Q>,
    phantom: PhantomData<(R, C)>,
}

impl<'q, Q: Route<R>, R: Route<Q>, C: External<'q, Q>> Session<'q, Q> for Branch<'q, Q, R, C>
where
    Q::Route: Receiver<R>,
    R: Role<Message = Q::Message>,
    R::Route: Sender<Q>,
{
    #[inline]
    fn from_state(state: State<'q, Q>) -> Self {
        Self {
            state,
            phantom: PhantomData,
        }
    }
}

impl<'q, Q: Route<R>, R: Route<Q>, C: External<'q, Q>> Branch<'q, Q, R, C>
where
    Q::Route: Receiver<R>,
    R: Role<Message = Q::Message>,
    R::Route: Sender<Q>,
{
    #[inline]
    pub async fn branch(self) -> Result<C, ReceiveError> {
        let message = self.state.0.route().receiver().receive().await?;
        C::choice(self.state, message).ok_or(ReceiveError::UnexpectedType)
    }
}

#[inline]
pub async fn session<'r, R: Role, S: Session<'r, R>, T, F>(
    role: &'r mut R,
    f: impl FnOnce(S) -> F,
) -> T
where
    F: Future<Output = (T, End<'r>)>,
{
    let output = try_session(role, |s| f(s).map(Ok)).await;
    output.unwrap_or_else(|infallible: Infallible| match infallible {})
}

#[inline]
pub async fn try_session<'r, R: Role, S: Session<'r, R>, T, E, F>(
    role: &'r mut R,
    f: impl FnOnce(S) -> F,
) -> Result<T, E>
where
    F: Future<Output = Result<(T, End<'r>), E>>,
{
    let state = State(role);
    f(state.into_session()).await.map(|(output, _)| output)
}
