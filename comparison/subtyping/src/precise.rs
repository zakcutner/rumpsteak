#![allow(clippy::nonstandard_macro_braces)]

use argh::FromArgs;
use rumpsteak::fsm::{
    self, Action, Dot, Local, Normalizer, Petrify, StateIndex, Transition, TransitionError,
};
use std::{result, str::FromStr};

type Fsm = fsm::Fsm<&'static str, &'static str>;

type Result<T, E = TransitionError> = result::Result<T, E>;

type Generate = fn(&mut Fsm, usize) -> Result<StateIndex>;

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
    /// number of levels to apply to the protocol
    #[argh(option, short = 'l', default = "0")]
    levels: usize,

    /// whether to reverse the subtype and supertype.
    #[argh(switch, short = 'r')]
    reverse: bool,

    /// which output format should be targeted
    #[argh(positional)]
    target: Target,
}

fn subtype(fsm: &mut Fsm, levels: usize) -> Result<StateIndex> {
    let s0 = fsm.add_state();
    if levels == 0 {
        return Ok(s0);
    }

    let s1 = fsm.add_state();
    let s2 = fsm.add_state();
    let s3 = subtype(fsm, levels - 1)?;
    let s4 = subtype(fsm, levels - 1)?;
    let s5 = subtype(fsm, levels - 1)?;
    let s6 = subtype(fsm, levels - 1)?;
    let s7 = subtype(fsm, levels - 1)?;

    fsm.add_transition(s0, s1, Transition::new("B", Action::Output, "m"))?;
    fsm.add_transition(s0, s2, Transition::new("B", Action::Output, "p"))?;
    fsm.add_transition(s1, s3, Transition::new("B", Action::Input, "r"))?;
    fsm.add_transition(s1, s4, Transition::new("B", Action::Input, "s"))?;
    fsm.add_transition(s1, s5, Transition::new("B", Action::Input, "u"))?;
    fsm.add_transition(s2, s6, Transition::new("B", Action::Input, "r"))?;
    fsm.add_transition(s2, s7, Transition::new("B", Action::Input, "s"))?;

    Ok(s0)
}

fn supertype(fsm: &mut Fsm, levels: usize) -> Result<StateIndex> {
    let s0 = fsm.add_state();
    if levels == 0 {
        return Ok(s0);
    }

    let s1 = fsm.add_state();
    let s2 = fsm.add_state();
    let s3 = subtype(fsm, levels - 1)?;
    let s4 = subtype(fsm, levels - 1)?;
    let s5 = subtype(fsm, levels - 1)?;
    let s6 = subtype(fsm, levels - 1)?;
    let s7 = subtype(fsm, levels - 1)?;

    fsm.add_transition(s0, s1, Transition::new("B", Action::Input, "r"))?;
    fsm.add_transition(s0, s2, Transition::new("B", Action::Input, "s"))?;
    fsm.add_transition(s1, s3, Transition::new("B", Action::Output, "m"))?;
    fsm.add_transition(s1, s4, Transition::new("B", Action::Output, "p"))?;
    fsm.add_transition(s1, s5, Transition::new("B", Action::Output, "q"))?;
    fsm.add_transition(s2, s6, Transition::new("B", Action::Output, "m"))?;
    fsm.add_transition(s2, s7, Transition::new("B", Action::Output, "p"))?;

    Ok(s0)
}

fn main() {
    let options = argh::from_env::<Options>();
    let (generate_left, generate_right): (Generate, Generate) = match options.reverse {
        false => (subtype, supertype),
        true => (supertype, subtype),
    };

    let mut left = Fsm::new("A");
    generate_left(&mut left, options.levels).unwrap();

    match options.target {
        Target::Rumpsteak => println!("{}", Dot::new(&left)),
        Target::Kmc => {
            let mut right = Fsm::new("A");
            generate_right(&mut right, options.levels).unwrap();

            let mut normalizer = Normalizer::default();
            let left = normalizer.normalize(&left);
            let right = normalizer.normalize(&right.dual("B"));

            println!("{}\n\n{}", Petrify::new(&left), Petrify::new(&right));
        }
        Target::Concur19 => println!("{}", Local::new(&left.to_binary())),
    }
}
