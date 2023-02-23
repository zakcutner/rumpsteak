use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional,
    session,
    Branch,
    End,
    Message,
    Receive,
    Role,
    Roles,
    Select,
    Send,
    effect::{
        SideEffect,
        Constant,
        Incr,
    },
    try_session,
    predicate::*,
    ParamName,
    Param,
};

use std::collections::HashMap;
use std::error::Error;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

type Name = char;
type Value = i32;

#[derive(Roles)]
#[allow(dead_code)]
struct Roles {
    c: C,
    a: A,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(A)]
    a: Channel,
}

#[derive(Role)]
#[message(Label)]
struct A {
    #[route(C)]
    c: Channel,
}

#[derive(Message, Copy, Clone)]
enum Label {
    Order(Order),
    Quote(Quote),
    Accept(Accept),
    Reject(Reject),
    Address(Address),
    Date(Date),
}

impl From<Label> for Value {
    fn from(label: Label) -> Value {
        match label {
            Label::Order(payload) => payload.into(),
            Label::Quote(payload) => payload.into(),
            Label::Accept(payload) => payload.into(),
            Label::Reject(payload) => payload.into(),
            Label::Address(payload) => payload.into(),
            Label::Date(payload) => payload.into(),
        }
    }
}


#[derive(Copy, Clone)]
struct Order(i32);

impl From<Order> for Value {
    fn from(value: Order) -> Value {
        let Order(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Quote(i32);

impl From<Quote> for Value {
    fn from(value: Quote) -> Value {
        let Quote(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Accept(i32);

impl From<Accept> for Value {
    fn from(value: Accept) -> Value {
        let Accept(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Reject(i32);

impl From<Reject> for Value {
    fn from(value: Reject) -> Value {
        let Reject(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Address(i32);

impl From<Address> for Value {
    fn from(value: Address) -> Value {
        let Address(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Date(i32);

impl From<Date> for Value {
    fn from(value: Date) -> Value {
        let Date(val) = value;
        val
    }
}

#[session(Name, Value)]
type TravelAgencyC = Send<A, 'o', Order, Tautology::<Name, Value, Order>, Constant<Name, Value>, Receive<A, 'q', Quote, Tautology::<Name, Value, Quote>, Constant<Name, Value>, Select<A, TravelAgencyC2Predicate, Constant<Name, Value>, TravelAgencyC2>>>;

#[session(Name, Value)]
enum TravelAgencyC2 {
    Reject(Reject, End),
    Accept(Accept, Send<A, 'a', Address, Tautology::<Name, Value, Address>, Constant<Name, Value>, Receive<A, 'd', Date, Tautology::<Name, Value, Date>, Constant<Name, Value>, End>>),
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for TravelAgencyC2<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            Label::Reject(Reject(val)) => {
                    ('n', *val)
            }
            Label::Accept(Accept(val)) => {
                    ('y', *val)
            }
            _ => panic!("Unexpected label"),
        }
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Reject, Name> for TravelAgencyC2<'__r, __R> {
    fn get_param_name() -> Name {
        'n'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Accept, Name> for TravelAgencyC2<'__r, __R> {
    fn get_param_name() -> Name {
        'y'
    }
}

#[derive(Default)]
struct TravelAgencyC2Predicate {}
impl Predicate for TravelAgencyC2Predicate {
    type Name = Name;
    type Value = Value;
    type Label = Label;
    type Error = ();

    fn check(
        &self,
        m: &HashMap<Self::Name, Self::Value>,
        label: Option<&Self::Label>
    ) -> Result<(), Self::Error> {
        if let Some(label) = label {
            match label {
                Label::Reject(_) => {
                    Tautology::<Name, Value, Label>::default()
                        .check(m, Some(label))
                    },
                Label::Accept(_) => {
                    LTnConst::<Label, 'q', 100>::default()
                        .check(m, Some(label))
                    },
                _ => {
                    Err(())
                }
            }
        } else {
            Err(())
        }
    }
}

#[session(Name, Value)]
type TravelAgencyA = Receive<C, 'o', Order, Tautology::<Name, Value, Order>, Constant<Name, Value>, Send<C, 'q', Quote, Tautology::<Name, Value, Quote>, Constant<Name, Value>, Branch<C, Tautology::<Name, Value, Label>, Constant<Name, Value>, TravelAgencyA2>>>;

#[session(Name, Value)]
enum TravelAgencyA2 {
    Reject(Reject, End),
    Accept(Accept, Receive<C, 'a', Address, Tautology::<Name, Value, Address>, Constant<Name, Value>, Send<C, 'd', Date, Tautology::<Name, Value, Date>, Constant<Name, Value>, End>>),
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for TravelAgencyA2<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            Label::Reject(Reject(val)) => {
                    ('n', *val)
            }
            Label::Accept(Accept(val)) => {
                    ('y', *val)
            }
            _ => panic!("Unexpected label"),
        }
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Reject, Name> for TravelAgencyA2<'__r, __R> {
    fn get_param_name() -> Name {
        'n'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Accept, Name> for TravelAgencyA2<'__r, __R> {
    fn get_param_name() -> Name {
        'y'
    }
}

#[derive(Default)]
struct TravelAgencyA2Predicate {}
impl Predicate for TravelAgencyA2Predicate {
    type Name = Name;
    type Value = Value;
    type Label = Label;
    type Error = ();

    fn check(
        &self,
        m: &HashMap<Self::Name, Self::Value>,
        label: Option<&Self::Label>
    ) -> Result<(), Self::Error> {
        if let Some(label) = label {
            match label {
                Label::Reject(_) => {
                    Tautology::<Name, Value, Label>::default()
                        .check(m, Some(label))
                    },
                Label::Accept(_) => {
                    LTnConst::<Label, 'q', 100>::default()
                        .check(m, Some(label))
                    },
                _ => {
                    Err(())
                }
            }
        } else {
            Err(())
        }
    }
}
