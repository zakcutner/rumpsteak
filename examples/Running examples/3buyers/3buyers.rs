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
    s: S,
}

#[derive(Role)]
#[message(Label)]
struct A {
    #[route(B)]
    b: Channel,
    #[route(S)]
    s: Channel,
}

#[derive(Role)]
#[message(Label)]
struct B {
    #[route(A)]
    a: Channel,
    #[route(S)]
    s: Channel,
}

#[derive(Role)]
#[message(Label)]
struct S {
    #[route(A)]
    a: Channel,
    #[route(B)]
    b: Channel,
}

#[derive(Message, Copy, Clone)]
enum Label {
    Request(Request),
    QuoteAlice(QuoteAlice),
    ParticipationBob(ParticipationBob),
    ConfirmAlice(ConfirmAlice),
    QuitAlice(QuitAlice),
    QuoteBob(QuoteBob),
    ConfirmSeller(ConfirmSeller),
    Date(Date),
    QuitSeller(QuitSeller),
}

impl From<Label> for Value {
    fn from(label: Label) -> Value {
        match label {
            Label::Request(payload) => payload.into(),
            Label::QuoteAlice(payload) => payload.into(),
            Label::ParticipationBob(payload) => payload.into(),
            Label::ConfirmAlice(payload) => payload.into(),
            Label::QuitAlice(payload) => payload.into(),
            Label::QuoteBob(payload) => payload.into(),
            Label::ConfirmSeller(payload) => payload.into(),
            Label::Date(payload) => payload.into(),
            Label::QuitSeller(payload) => payload.into(),
        }
    }
}


#[derive(Copy, Clone)]
struct Request(i32);

impl From<Request> for Value {
    fn from(value: Request) -> Value {
        let Request(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct QuoteAlice(i32);

impl From<QuoteAlice> for Value {
    fn from(value: QuoteAlice) -> Value {
        let QuoteAlice(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct ParticipationBob(i32);

impl From<ParticipationBob> for Value {
    fn from(value: ParticipationBob) -> Value {
        let ParticipationBob(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct ConfirmAlice(i32);

impl From<ConfirmAlice> for Value {
    fn from(value: ConfirmAlice) -> Value {
        let ConfirmAlice(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct QuitAlice(i32);

impl From<QuitAlice> for Value {
    fn from(value: QuitAlice) -> Value {
        let QuitAlice(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct QuoteBob(i32);

impl From<QuoteBob> for Value {
    fn from(value: QuoteBob) -> Value {
        let QuoteBob(val) = value;
        val
    }
}

#[derive(Copy, Clone)]
struct ConfirmSeller(i32);

impl From<ConfirmSeller> for Value {
    fn from(value: ConfirmSeller) -> Value {
        let ConfirmSeller(val) = value;
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

#[derive(Copy, Clone)]
struct QuitSeller(i32);

impl From<QuitSeller> for Value {
    fn from(value: QuitSeller) -> Value {
        let QuitSeller(val) = value;
        val
    }
}

#[session(Name, Value)]
type ThreeBuyersA = Send<S, 'r', Request, Tautology::<Name, Value, Request>, Constant<Name, Value>, Receive<S, 'a', QuoteAlice, Tautology::<Name, Value, QuoteAlice>, Constant<Name, Value>, Send<B, 'p', ParticipationBob, Tautology::<Name, Value, ParticipationBob>, Constant<Name, Value>, Branch<B, Tautology::<Name, Value, Label>, Constant<Name, Value>, ThreeBuyersA3>>>>;

#[session(Name, Value)]
enum ThreeBuyersA3 {
    QuitAlice(QuitAlice, End),
    ConfirmAlice(ConfirmAlice, End),
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for ThreeBuyersA3<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            Label::QuitAlice(QuitAlice(val)) => {
                    ('c', *val)
            }
            Label::ConfirmAlice(ConfirmAlice(val)) => {
                    ('c', *val)
            }
            _ => panic!("Unexpected label"),
        }
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<QuitAlice, Name> for ThreeBuyersA3<'__r, __R> {
    fn get_param_name() -> Name {
        'c'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<ConfirmAlice, Name> for ThreeBuyersA3<'__r, __R> {
    fn get_param_name() -> Name {
        'c'
    }
}

#[derive(Default)]
struct ThreeBuyersA3Predicate {}
impl Predicate for ThreeBuyersA3Predicate {
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
                Label::QuitAlice(_) => {
                    Tautology::<Name, Value, Label>::default()
                        .check(m, Some(label))
                    },
                Label::ConfirmAlice(_) => {
                    Tautology::<Name, Value, Label>::default()
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
type ThreeBuyersB = Receive<S, 'b', QuoteBob, Tautology::<Name, Value, QuoteBob>, Constant<Name, Value>, Receive<A, 'p', ParticipationBob, Tautology::<Name, Value, ParticipationBob>, Constant<Name, Value>, Select<A, ThreeBuyersB2Predicate, Constant<Name, Value>, ThreeBuyersB2>>>;

#[session(Name, Value)]
enum ThreeBuyersB2 {
    QuitAlice(QuitAlice, Send<S, 's', QuitSeller, Tautology::<Name, Value, QuitSeller>, Constant<Name, Value>, End>),
    ConfirmAlice(ConfirmAlice, Send<S, 's', ConfirmSeller, Tautology::<Name, Value, ConfirmSeller>, Constant<Name, Value>, Receive<S, 'd', Date, Tautology::<Name, Value, Date>, Constant<Name, Value>, End>>),
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for ThreeBuyersB2<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            Label::QuitAlice(QuitAlice(val)) => {
                    ('c', *val)
            }
            Label::ConfirmAlice(ConfirmAlice(val)) => {
                    ('c', *val)
            }
            _ => panic!("Unexpected label"),
        }
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<QuitAlice, Name> for ThreeBuyersB2<'__r, __R> {
    fn get_param_name() -> Name {
        'c'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<ConfirmAlice, Name> for ThreeBuyersB2<'__r, __R> {
    fn get_param_name() -> Name {
        'c'
    }
}

#[derive(Default)]
struct ThreeBuyersB2Predicate {}
impl Predicate for ThreeBuyersB2Predicate {
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
                Label::QuitAlice(_) => {
                    Tautology::<Name, Value, Label>::default()
                        .check(m, Some(label))
                    },
                Label::ConfirmAlice(_) => {
                    Tautology::<Name, Value, Label>::default()
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
type ThreeBuyersS = Receive<A, 'r', Request, Tautology::<Name, Value, Request>, Constant<Name, Value>, Send<A, 'a', QuoteAlice, Tautology::<Name, Value, QuoteAlice>, Constant<Name, Value>, Send<B, 'b', QuoteBob, Tautology::<Name, Value, QuoteBob>, Constant<Name, Value>, Branch<B, Tautology::<Name, Value, Label>, Constant<Name, Value>, ThreeBuyersS3>>>>;

#[session(Name, Value)]
enum ThreeBuyersS3 {
    ConfirmSeller(ConfirmSeller, Send<B, 'd', Date, Tautology::<Name, Value, Date>, Constant<Name, Value>, End>),
    QuitSeller(QuitSeller, End),
}

impl<'__r, __R: ::rumpsteak::Role> Param<Name, Value, Label> for ThreeBuyersS3<'__r, __R> {
    fn get_param(l: &Label) -> (Name, Value) {
        match l {
            Label::ConfirmSeller(ConfirmSeller(val)) => {
                    ('s', *val)
            }
            Label::QuitSeller(QuitSeller(val)) => {
                    ('s', *val)
            }
            _ => panic!("Unexpected label"),
        }
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<ConfirmSeller, Name> for ThreeBuyersS3<'__r, __R> {
    fn get_param_name() -> Name {
        's'
    }
}
impl<'__r, __R: ::rumpsteak::Role> ParamName<QuitSeller, Name> for ThreeBuyersS3<'__r, __R> {
    fn get_param_name() -> Name {
        's'
    }
}

#[derive(Default)]
struct ThreeBuyersS3Predicate {}
impl Predicate for ThreeBuyersS3Predicate {
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
                Label::ConfirmSeller(_) => {
                    Tautology::<Name, Value, Label>::default()
                        .check(m, Some(label))
                    },
                Label::QuitSeller(_) => {
                    Tautology::<Name, Value, Label>::default()
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

async fn b(role: &mut B) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: ThreeBuyersB<'_, _>| async {
        let (QuoteBob(msg_s), s) = s.receive().await?;
        let (ParticipationBob(msg_a), s) = s.receive().await?;
        if msg_a == msg_s {
            // Accept command if both prices are the same
            let s = s.select(ConfirmAlice(msg_s)).await?;
            let s = s.send(ConfirmSeller(msg_s)).await?;
            let (Date(_msg), s) = s.receive().await?;
            println!("Accept order (price {})", msg_a);
            Ok(((), s))
        } else {
            let s = s.select(QuitAlice(0)).await?;
            let s = s.send(QuitSeller(0)).await?;
            println!("Reject order (price inconsistency {} vs {})", msg_a, msg_s);
            Ok(((), s))
        }
    })
    .await
}

async fn s(role: &mut S) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: ThreeBuyersS<'_, _>| async {
        let (Request(msg), s) = s.receive().await?;
        let s = s.send(QuoteAlice(msg)).await?;
        let s = s.send(QuoteBob(msg)).await?;
        match s.branch().await? {
            ThreeBuyersS3::ConfirmSeller(msg, s) => {
                let s = s.send(Date(10)).await?;
                Ok(((), s))
            }
            ThreeBuyersS3::QuitSeller(_, end) => Ok(((), end)),
        }
    })
    .await
}

async fn a(role: &mut A) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(), |s: ThreeBuyersA<'_, _>| async {
        let s = s.send(Request(42)).await?;
        let (_reply, s) = s.receive().await?;
        let s = s.send(ParticipationBob(42)).await?;
        match s.branch().await? {
            ThreeBuyersA3::ConfirmAlice(_, end) => Ok(((), end)),
            ThreeBuyersA3::QuitAlice(_, end) => Ok(((), end)),
        }
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(b(&mut roles.b), s(&mut roles.s), a(&mut roles.a)).unwrap();
    });
}
