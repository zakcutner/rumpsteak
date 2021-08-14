#![allow(clippy::nonstandard_macro_braces)]

use argh::FromArgs;
use rumpsteak::fsm::{self, Action, Dot, Normalizer, Petrify, Transition, TransitionError};
use std::{error::Error, result, str::FromStr};

type Fsm = fsm::Fsm<&'static str, &'static str>;

type Result<T, E = TransitionError> = result::Result<T, E>;

struct Roles(usize);

impl FromStr for Roles {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let roles = s.parse::<usize>()?;
        if roles < 2 {
            return Err("there cannot be less than two roles".into());
        }

        Ok(Self(roles))
    }
}

enum Target {
    Rumpsteak,
    Kmc,
}

impl FromStr for Target {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rumpsteak" => Ok(Self::Rumpsteak),
            "kmc" => Ok(Self::Kmc),
            _ => Err("invalid target choice, possible values are 'rumpsteak' or 'kmc'"),
        }
    }
}

#[derive(FromArgs)]
/// Generate finite state machines for the double buffering protocol.
struct Options {
    /// number of unrolls to apply to the kernel
    #[argh(option, short = 'u', default = "0")]
    unrolls: usize,

    /// which output format should be targeted
    #[argh(positional)]
    target: Target,
}

fn source() -> Result<Fsm> {
    let mut fsm = Fsm::new("S");

    let s0 = fsm.add_state();
    let s1 = fsm.add_state();

    fsm.add_transition(s0, s1, Transition::new("K", Action::Input, "ready"))?;
    fsm.add_transition(s1, s0, Transition::new("K", Action::Output, "value"))?;

    Ok(fsm)
}

fn kernel(unrolls: usize) -> Result<Fsm> {
    let mut fsm = Fsm::new("K");
    let mut s0 = fsm.add_state();

    for _ in 0..unrolls {
        let s1 = fsm.add_state();
        fsm.add_transition(s0, s1, Transition::new("S", Action::Output, "ready"))?;
        s0 = s1;
    }

    let s1 = fsm.add_state();
    let s2 = fsm.add_state();
    let s3 = fsm.add_state();

    fsm.add_transition(s0, s1, Transition::new("S", Action::Output, "ready"))?;
    fsm.add_transition(s1, s2, Transition::new("S", Action::Input, "value"))?;
    fsm.add_transition(s2, s3, Transition::new("T", Action::Input, "ready"))?;
    fsm.add_transition(s3, s0, Transition::new("T", Action::Output, "value"))?;

    Ok(fsm)
}

fn sink() -> Result<Fsm> {
    let mut fsm = Fsm::new("T");

    let s0 = fsm.add_state();
    let s1 = fsm.add_state();

    fsm.add_transition(s0, s1, Transition::new("K", Action::Output, "ready"))?;
    fsm.add_transition(s1, s0, Transition::new("K", Action::Input, "value"))?;

    Ok(fsm)
}

fn main() {
    let options = argh::from_env::<Options>();
    let kernel = kernel(options.unrolls).unwrap();

    match options.target {
        Target::Rumpsteak => {
            println!("{}", Dot::new(&kernel));
        }
        Target::Kmc => {
            let mut normalizer = Normalizer::default();

            let source = source().unwrap();
            let source = normalizer.normalize(&source);

            let kernel = normalizer.normalize(&kernel);

            let sink = sink().unwrap();
            let sink = normalizer.normalize(&sink);

            println!(
                "{}\n\n{}\n\n{}",
                Petrify::new(&source),
                Petrify::new(&kernel),
                Petrify::new(&sink)
            );
        }
    }
}
