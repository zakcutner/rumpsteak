#![allow(clippy::nonstandard_macro_braces)]

use argh::FromArgs;
use rumpsteak::fsm::{
    self,
    local::{Binary, Type},
    Action, Dot, Normalizer, Petrify, Transition, TransitionError,
};
use std::{result, str::FromStr};

type Fsm = fsm::Fsm<&'static str, &'static str>;

type Result<T, E = TransitionError> = result::Result<T, E>;

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
        fsm.add_transition(s0, s1, Transition::new("T", Action::Output, "value"))?;
        s0 = s1;
    }

    for _ in 0..unrolls {
        let s1 = fsm.add_state();
        fsm.add_transition(s0, s1, Transition::new("T", Action::Input, "ready"))?;
        s0 = s1;
    }

    let s1 = fsm.add_state();
    let s2 = fsm.add_state();

    fsm.add_transition(s0, s1, Transition::new("T", Action::Input, "ready"))?;
    fsm.add_transition(s1, s0, Transition::new("T", Action::Output, "value"))?;
    fsm.add_transition(s1, s2, Transition::new("T", Action::Output, "stop"))?;

    Ok(fsm)
}

fn sink() -> Result<Fsm> {
    let mut fsm = Fsm::new("T");

    let s0 = fsm.add_state();
    let s1 = fsm.add_state();
    let s2 = fsm.add_state();

    fsm.add_transition(s0, s1, Transition::new("S", Action::Output, "ready"))?;
    fsm.add_transition(s1, s0, Transition::new("S", Action::Input, "value"))?;
    fsm.add_transition(s1, s2, Transition::new("S", Action::Input, "stop"))?;

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
        Target::Concur19 => println!("{}", Binary::new(&Type::new(&source))),
    }
}
