use futures::{channel::mpsc, StreamExt};
use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct SendError(#[from] mpsc::SendError);

pub struct Sender<T>(mpsc::UnboundedSender<T>);

impl<T> Sender<T> {
    #[inline]
    pub fn send(&mut self, message: T) -> Result<(), SendError> {
        let result = self.0.unbounded_send(message);
        result.map_err(|err| SendError::from(mpsc::TrySendError::into_send_error(err)))
    }
}

#[derive(Debug, Error)]
#[error("receiver channel is empty")]
pub struct ReceiveError;

pub struct Receiver<T>(mpsc::UnboundedReceiver<T>);

impl<T> Receiver<T> {
    #[inline]
    pub async fn receive(&mut self) -> Result<T, ReceiveError> {
        let message = StreamExt::next(&mut self.0).await;
        message.ok_or(ReceiveError)
    }
}

pub(crate) fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let (sender, receiver) = mpsc::unbounded();
    (Sender(sender), Receiver(receiver))
}
