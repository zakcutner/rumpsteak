pub use rumpsteak_macros::{Role, Roles};

use crate::channel;
use std::marker::PhantomData;

pub trait Role {
    type Message;
}

pub trait Pair<P: Pair<Self> + Role<Message = Self::Message>>: Role + Sized {
    fn pair() -> (Self, P);
}

pub trait Route<R: Route<Self> + Role<Message = Self::Message>>: Role + Sized {
    type Route: Pair<R::Route> + Role<Message = Self::Message>;

    fn route(&mut self) -> &mut Self::Route;
}

pub trait Sender<R: Role<Message = Self::Message>>: Role {
    fn sender(&mut self) -> &mut channel::Sender<Self::Message>;
}

pub trait Receiver<R: Role<Message = Self::Message>>: Role {
    fn receiver(&mut self) -> &mut channel::Receiver<Self::Message>;
}

pub struct Nil<R: Role>(PhantomData<R>);

impl<R: Role> Role for Nil<R> {
    type Message = R::Message;
}

impl<Q: Role, R: Role<Message = Q::Message>> Pair<Nil<R>> for Nil<Q> {
    #[inline]
    fn pair() -> (Nil<Q>, Nil<R>) {
        (Nil(PhantomData), Nil(PhantomData))
    }
}

pub struct To<R: Role>(channel::Sender<R::Message>);

impl<R: Role> Role for To<R> {
    type Message = R::Message;
}

impl<Q: Role, R: Role<Message = Q::Message>> Pair<From<R>> for To<Q> {
    #[inline]
    fn pair() -> (To<Q>, From<R>) {
        let (to, from) = channel::channel();
        (To(to), From(from))
    }
}

impl<R: Role> Sender<R> for To<R> {
    #[inline]
    fn sender(&mut self) -> &mut channel::Sender<R::Message> {
        &mut self.0
    }
}

pub struct From<R: Role>(channel::Receiver<R::Message>);

impl<R: Role> Role for From<R> {
    type Message = R::Message;
}

impl<Q: Role, R: Role<Message = Q::Message>> Pair<To<R>> for From<Q> {
    #[inline]
    fn pair() -> (From<Q>, To<R>) {
        let (to, from) = channel::channel();
        (From(from), To(to))
    }
}

impl<R: Role> Receiver<R> for From<R> {
    #[inline]
    fn receiver(&mut self) -> &mut channel::Receiver<R::Message> {
        &mut self.0
    }
}

pub struct ToFrom<R: Role>(To<R>, From<R>);

impl<R: Role> Role for ToFrom<R> {
    type Message = R::Message;
}

impl<Q: Role, R: Role<Message = Q::Message>> Pair<ToFrom<R>> for ToFrom<Q> {
    #[inline]
    fn pair() -> (ToFrom<Q>, ToFrom<R>) {
        let (left_to, left_from) = Pair::pair();
        let (right_to, right_from) = Pair::pair();
        (ToFrom(left_to, right_from), ToFrom(right_to, left_from))
    }
}

impl<R: Role> Sender<R> for ToFrom<R> {
    #[inline]
    fn sender(&mut self) -> &mut channel::Sender<R::Message> {
        self.0.sender()
    }
}

impl<R: Role> Receiver<R> for ToFrom<R> {
    #[inline]
    fn receiver(&mut self) -> &mut channel::Receiver<R::Message> {
        self.1.receiver()
    }
}
