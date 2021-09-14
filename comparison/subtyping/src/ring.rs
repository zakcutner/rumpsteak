#![allow(clippy::nonstandard_macro_braces)]

use argh::FromArgs;
use rumpsteak::fsm::{self, Action, AddTransitionError, Dot, Message, Petrify, Transition};
use std::{convert::Infallible, error::Error, iter, result, str::FromStr};

type Fsm = fsm::Fsm<usize, usize, Infallible>;

type Result<T, E = AddTransitionError> = result::Result<T, E>;

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
/// Generate finite state machines for the ring protocol.
struct Options {
    /// number of roles in the protocol
    #[argh(option, short = 'r', default = "Roles(2)")]
    roles: Roles,

    /// whether to optimize each role.
    #[argh(switch, short = 'O')]
    optimized: bool,

    /// which output format should be targeted
    #[argh(positional)]
    target: Target,
}

fn unoptimized(n: usize, before: usize, after: usize) -> Result<Fsm> {
    let mut fsm = Fsm::new(n);

    let s0 = fsm.add_state();
    let s1 = fsm.add_state();

    let transition = Transition::new(before, Action::Input, Message::from_label(0));
    fsm.add_transition(s0, s1, transition)?;

    let transition = Transition::new(after, Action::Output, Message::from_label(0));
    fsm.add_transition(s1, s0, transition)?;

    Ok(fsm)
}

fn optimized(n: usize, before: usize, after: usize) -> Result<Fsm> {
    let mut fsm = Fsm::new(n);

    let s0 = fsm.add_state();
    let s1 = fsm.add_state();

    let transition = Transition::new(after, Action::Output, Message::from_label(0));
    fsm.add_transition(s0, s1, transition)?;

    let transition = Transition::new(before, Action::Input, Message::from_label(0));
    fsm.add_transition(s1, s0, transition)?;

    Ok(fsm)
}

fn main() {
    let options = argh::from_env::<Options>();
    let Roles(roles) = options.roles;

    let mut fsms = (1..roles - 1)
        .map(|n| (n, n - 1, n + 1))
        .chain(iter::once((roles - 1, roles - 2, 0)))
        .map(|(n, before, after)| match options.optimized {
            true => optimized(n, before, after).unwrap(),
            false => unoptimized(n, before, after).unwrap(),
        });

    match options.target {
        Target::Rumpsteak => {
            println!("{}", Dot::new(&fsms.next().unwrap()));
            for fsm in fsms {
                println!("\n{}", Dot::new(&fsm));
            }
        }
        Target::Kmc => {
            println!("{}", Petrify::new(&optimized(0, roles - 1, 1).unwrap()));
            for fsm in fsms {
                println!("\n{}", Petrify::new(&fsm));
            }
        }
    }
}
