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
    a: A,
    b: B,
    c: C,
}

#[derive(Role)]
#[message(Label)]
struct A {
    #[route(B)]
    b: Channel,
    #[route(C)]
    c: Channel,
}

#[derive(Role)]
#[message(Label)]
struct B {
    #[route(A)]
    a: Channel,
    #[route(C)]
    c: Channel,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(A)]
    a: Channel,
    #[route(B)]
    b: Channel,
}

#[derive(Message, Copy, Clone)]
enum Label {
    Secret(Secret),
    Guess(Guess),
    Less(Less),
    More(More),
    Correct(Correct),
}

impl From<Label> for Value {
    fn from(label: Label) -> Value {
        match label {
            Label::Secret(payload) => payload.into(),
            Label::Guess(payload) => payload.into(),
            Label::Less(payload) => payload.into(),
            Label::More(payload) => payload.into(),
            Label::Correct(payload) => payload.into(),
        }
    }
}


#[derive(Copy, Clone)]
struct Secret(i32);

impl From<Secret> for Value {
    fn from(value: Secret) -> Value {
        let Secret(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Guess(i32);

impl From<Guess> for Value {
    fn from(value: Guess) -> Value {
        let Guess(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Less(i32);

impl From<Less> for Value {
    fn from(value: Less) -> Value {
        let Less(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct More(i32);

impl From<More> for Value {
    fn from(value: More) -> Value {
        let More(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct Correct(i32);

impl From<Correct> for Value {
    fn from(value: Correct) -> Value {
        let Correct(val) = value;
        val
    }
}

#[session(Name, Value)]
type PlusMinusA = Send<B, 'n', Secret, Tautology::<Name, Value, Secret>, Constant<Name, Value>, End>;

#[session(Name, Value)]
type PlusMinusB = Receive<A, 'n', Secret, Tautology::<Name, Value, Secret>, Constant<Name, Value>, PlusMinusB1>;

#[session(Name, Value)]
type PlusMinusB1 = Receive<C, 'x', Guess, Tautology::<Name, Value, Guess>, Constant<Name, Value>, Select<C, PlusMinusB3Predicate, Constant<Name, Value>, PlusMinusB3>>;

#[session(Name, Value)]
enum PlusMinusB3 {
    Correct(Correct, End),
    More(More, PlusMinusB1),
    Less(Less, PlusMinusB1),
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for PlusMinusB3<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            Label::Correct(Correct(val)) => {
                    ('x', *val)
            }
            Label::More(More(val)) => {
                    ('x', *val)
            }
            Label::Less(Less(val)) => {
                    ('x', *val)
            }
            _ => panic!("Unexpected label"),
        }
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Correct, Name> for PlusMinusB3<'__r, __R> {
    fn get_param_name() -> Name {
        'x'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<More, Name> for PlusMinusB3<'__r, __R> {
    fn get_param_name() -> Name {
        'x'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Less, Name> for PlusMinusB3<'__r, __R> {
    fn get_param_name() -> Name {
        'x'
    }
}

#[derive(Default)]
struct PlusMinusB3Predicate {}
impl Predicate for PlusMinusB3Predicate {
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
                Label::Correct(_) => {
                    EqualVar::<Value, Label, 'x', 'n'>::default()
                        .check(m, Some(label))
                    },
                Label::More(_) => {
                    LTnVar::<Value, Label, 'x', 'n'>::default()
                        .check(m, Some(label))
                    },
                Label::Less(_) => {
                    GTnVar::<Value, Label, 'x', 'n'>::default()
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
type PlusMinusC = Send<B, 'x', Guess, Tautology::<Name, Value, Guess>, Constant<Name, Value>, Branch<B, Tautology::<Name, Value, Label>, Constant<Name, Value>, PlusMinusC2>>;

#[session(Name, Value)]
enum PlusMinusC2 {
    Correct(Correct, End),
    More(More, PlusMinusC),
    Less(Less, PlusMinusC),
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for PlusMinusC2<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            Label::Correct(Correct(val)) => {
                    ('x', *val)
            }
            Label::More(More(val)) => {
                    ('x', *val)
            }
            Label::Less(Less(val)) => {
                    ('x', *val)
            }
            _ => panic!("Unexpected label"),
        }
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Correct, Name> for PlusMinusC2<'__r, __R> {
    fn get_param_name() -> Name {
        'x'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<More, Name> for PlusMinusC2<'__r, __R> {
    fn get_param_name() -> Name {
        'x'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<Less, Name> for PlusMinusC2<'__r, __R> {
    fn get_param_name() -> Name {
        'x'
    }
}

#[derive(Default)]
struct PlusMinusC2Predicate {}
impl Predicate for PlusMinusC2Predicate {
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
                Label::Correct(_) => {
                    EqualVar::<Value, Label, 'x', 'n'>::default()
                        .check(m, Some(label))
                    },
                Label::More(_) => {
                    LTnVar::<Value, Label, 'x', 'n'>::default()
                        .check(m, Some(label))
                    },
                Label::Less(_) => {
                    GTnVar::<Value, Label, 'x', 'n'>::default()
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
