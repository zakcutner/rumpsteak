#![allow(clippy::nonstandard_macro_braces)]

use argh::FromArgs;
use rumpsteak_fsm::{
    Action, AddTransitionError, Dot, Local, Message, Normalizer, Petrify, Transition,
};
use std::{convert::Infallible, result, str::FromStr};

type Fsm = rumpsteak_fsm::Fsm<&'static str, &'static str, Infallible>;

type Result<T, E = AddTransitionError> = result::Result<T, E>;

enum Target {
    Rumpsteak,
    Kmc,
    Concur19,
}

impl FromStr for Target {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rumpsteak" => Ok(Self::Rumpsteak),
            "kmc" => Ok(Self::Kmc),
            "concur19" => Ok(Self::Concur19),
            _ => Err("invalid target choice, possible values are 'rumpsteak', 'kmc' or 'concur19'"),
        }
    }
}

#[derive(FromArgs)]
/// Generate finite state machines for the streaming protocol.
struct Options {
    /// number of unrolls to apply to the source
    #[argh(option, short = 'u', default = "0")]
    unrolls: usize,

    /// which output format should be targeted
    #[argh(positional)]
    target: Target,
}

fn source(unrolls: usize) -> Result<Fsm> {
    let mut fsm = Fsm::new("S");
    let mut s0 = fsm.add_state();

    for _ in 0..unrolls {
        let s1 = fsm.add_state();
        let transition = Transition::new("T", Action::Output, Message::from_label("value"));
        fsm.add_transition(s0, s1, transition)?;
        s0 = s1;
    }

    for _ in 0..unrolls {
        let s1 = fsm.add_state();
        let transition = Transition::new("T", Action::Input, Message::from_label("ready"));
        fsm.add_transition(s0, s1, transition)?;
        s0 = s1;
    }

    let s1 = fsm.add_state();
    let s2 = fsm.add_state();

    let transition = Transition::new("T", Action::Input, Message::from_label("ready"));
    fsm.add_transition(s0, s1, transition)?;

    let transition = Transition::new("T", Action::Output, Message::from_label("value"));
    fsm.add_transition(s1, s0, transition)?;

    let transition = Transition::new("T", Action::Output, Message::from_label("stop"));
    fsm.add_transition(s1, s2, transition)?;

    Ok(fsm)
}

fn sink() -> Result<Fsm> {
    let mut fsm = Fsm::new("T");

    let s0 = fsm.add_state();
    let s1 = fsm.add_state();
    let s2 = fsm.add_state();

    let transition = Transition::new("S", Action::Output, Message::from_label("ready"));
    fsm.add_transition(s0, s1, transition)?;

    let transition = Transition::new("S", Action::Input, Message::from_label("value"));
    fsm.add_transition(s1, s0, transition)?;

    let transition = Transition::new("S", Action::Input, Message::from_label("stop"));
    fsm.add_transition(s1, s2, transition)?;

    Ok(fsm)
}

fn main() {
    let options = argh::from_env::<Options>();
    let source = source(options.unrolls).unwrap();

    match options.target {
        Target::Rumpsteak => println!("{}", Dot::new(&source)),
        Target::Kmc => {
            let mut normalizer = Normalizer::default();
            let source = normalizer.normalize(&source);
            let sink = normalizer.normalize(&sink().unwrap());
            println!("{}\n\n{}", Petrify::new(&source), Petrify::new(&sink));
        }
        Target::Concur19 => println!("{}", Local::new(&source.to_binary())),
    }
}
