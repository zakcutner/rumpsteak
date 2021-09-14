use futures::{channel::mpsc, Sink, Stream};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub trait Pair<P: Pair<Self>>: Sized {
    fn pair() -> (Self, P);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nil;

impl Pair<Nil> for Nil {
    #[inline]
    fn pair() -> (Nil, Nil) {
        (Nil, Nil)
    }
}

impl<T> Pair<mpsc::UnboundedReceiver<T>> for mpsc::UnboundedSender<T> {
    fn pair() -> (Self, mpsc::UnboundedReceiver<T>) {
        mpsc::unbounded()
    }
}

impl<T> Pair<mpsc::UnboundedSender<T>> for mpsc::UnboundedReceiver<T> {
    fn pair() -> (Self, mpsc::UnboundedSender<T>) {
        let (sender, receiver) = Pair::pair();
        (receiver, sender)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bidirectional<S, R> {
    sender: S,
    receiver: R,
}

impl<S, R> Bidirectional<S, R> {
    pub fn new(sender: S, receiver: R) -> Self {
        Self { sender, receiver }
    }
}

impl<S: Pair<R>, R: Pair<S>> Pair<Self> for Bidirectional<S, R> {
    fn pair() -> (Self, Self) {
        let (left_sender, right_receiver) = Pair::pair();
        let (right_sender, left_receiver) = Pair::pair();
        (
            Bidirectional::new(left_sender, left_receiver),
            Bidirectional::new(right_sender, right_receiver),
        )
    }
}

impl<S: Unpin, R: Unpin> Bidirectional<S, R> {
    fn sender(self: Pin<&mut Self>) -> Pin<&mut S> {
        Pin::new(&mut self.get_mut().sender)
    }

    fn receiver(self: Pin<&mut Self>) -> Pin<&mut R> {
        Pin::new(&mut self.get_mut().receiver)
    }
}

impl<T, S: Sink<T> + Unpin, R: Unpin> Sink<T> for Bidirectional<S, R> {
    type Error = S::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        S::poll_ready(self.sender(), cx)
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        S::start_send(self.sender(), item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        S::poll_flush(self.sender(), cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        S::poll_close(self.sender(), cx)
    }
}

impl<S: Unpin, R: Stream + Unpin> Stream for Bidirectional<S, R> {
    type Item = R::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        R::poll_next(self.receiver(), cx)
    }
}
