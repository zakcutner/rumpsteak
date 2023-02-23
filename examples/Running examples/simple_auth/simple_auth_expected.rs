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
    s: S,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(S)]
    s: Channel,
}

#[derive(Role)]
#[message(Label)]
struct S {
    #[route(C)]
    c: Channel,
}

#[derive(Message, Copy, Clone)]
enum Label {
    SetPw(SetPw),
    Password(Password),
    Success(Success),
    Failure(Failure),
    RetX(RetX),
    RetRes(RetRes),
}

impl From<Label> for Value {
    fn from(label: Label) -> Value {
        match label {
            Label::SetPw(payload) => payload.into(),
            Label::Password(payload) => payload.into(),
            Label::Success(payload) => payload.into(),
            Label::Failure(payload) => payload.into(),
            Label::RetX(payload) => payload.into(),
            Label::RetRes(payload) => payload.into(),
        }
    }
}


#[derive(Copy, Clone)]
struct SetPw(i32);

impl From<SetPw> for Value {
    fn from(value: SetPw) -> Value {
        let SetPw(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Password(i32);

impl From<Password> for Value {
    fn from(value: Password) -> Value {
        let Password(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Success(i32);

impl From<Success> for Value {
    fn from(value: Success) -> Value {
        let Success(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Failure(i32);

impl From<Failure> for Value {
    fn from(value: Failure) -> Value {
        let Failure(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct RetX(i32);

impl From<RetX> for Value {
    fn from(value: RetX) -> Value {
        let RetX(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct RetRes(i32);

impl From<RetRes> for Value {
    fn from(value: RetRes) -> Value {
        let RetRes(val) = value;
        val
    }
}

#[session(Name, Value)]
type AuthC = Send<S, 'p', SetPw, Tautology<Name, Value, SetPw>, Constant<Name, Value>, AuthC1>;

#[session(Name, Value)]
type AuthC1 = Send<S, 'x', Password, Tautology<Name, Value, Password>, Constant<Name, Value>, Branch<S, Tautology<Name, Value, Label>, Constant<Name, Value>, AuthC3>>;

#[session(Name, Value)]
enum AuthC3 {
    Failure(Failure, Receive<S, 'x', RetX, Tautology<Name, Value, RetX>, Constant<Name, Value>, Send<S, 'r', RetRes, Tautology<Name, Value, RetRes>, Constant<Name, Value>, AuthC1>>),
    Success(Success, End),
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for AuthC3<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            Label::Failure(Failure(val)) => {
                    ('r', *val)
            }
            Label::Success(Success(val)) => {
                    ('r', *val)
            }
            _ => panic!("Unexpected label"),
        }
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Failure, Name> for AuthC3<'__r, __R> {
    fn get_param_name() -> Name {
        'r'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Success, Name> for AuthC3<'__r, __R> {
    fn get_param_name() -> Name {
        'r'
    }
}

#[derive(Default)]
struct AuthC3Predicate {}
impl Predicate for AuthC3Predicate {
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
                Label::Failure(_) => {
                    Neg::<Label, EqualVar::<Value, Label, 'x', 'p'>, Name, Value>::default()
                        .check(m, Some(label))
                    },
                Label::Success(_) => {
                    EqualVar::<Value, Label, 'x', 'p'>::default()
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
type AuthS = Receive<C, 'p', SetPw, Tautology<Name, Value, SetPw>, Constant<Name, Value>, AuthS1>;

#[session(Name, Value)]
type AuthS1 = Receive<C, 'x', Password, Tautology<Name, Value, Password>, Constant<Name, Value>, Select<C, AuthS3Predicate, Constant<Name, Value>, AuthS3>>;

#[session(Name, Value)]
enum AuthS3 {
    Failure(Failure, Send<C, 'x', RetX, Tautology<Name, Value, RetX>, Constant<Name, Value>, Receive<C, 'r', RetRes, Tautology<Name, Value, RetRes>, Constant<Name, Value>, AuthS1>>),
    Success(Success, End),
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for AuthS3<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            Label::Failure(Failure(val)) => {
                    ('r', *val)
            }
            Label::Success(Success(val)) => {
                    ('r', *val)
            }
            _ => panic!("Unexpected label"),
        }
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Failure, Name> for AuthS3<'__r, __R> {
    fn get_param_name() -> Name {
        'r'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Success, Name> for AuthS3<'__r, __R> {
    fn get_param_name() -> Name {
        'r'
    }
}

#[derive(Default)]
struct AuthS3Predicate {}
impl Predicate for AuthS3Predicate {
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
                Label::Failure(_) => {
                    Neg::<Label, EqualVar::<Value, Label, 'x', 'p'>, Name, Value>::default()
                        .check(m, Some(label))
                    },
                Label::Success(_) => {
                    EqualVar::<Value, Label, 'x', 'p'>::default()
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
