mod parse;

#[cfg(feature = "parsing")]
pub use self::parse::{parse, ParseError, ParseIter};

use super::Fsm;
use std::fmt::{self, Display, Formatter};

pub struct Dot<'a, R, L>(&'a Fsm<R, L>);

impl<'a, R, L> Dot<'a, R, L> {
    pub fn new(fsm: &'a Fsm<R, L>) -> Self {
        Self(fsm)
    }
}

impl<'a, R: Display, L: Display> Display for Dot<'a, R, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "digraph \"{}\" {{", self.0.role())?;
        let (states, transitions) = self.0.size();

        if states > 0 {
            writeln!(f)?;
        }

        for i in self.0.states() {
            writeln!(f, "    {};", i.index())?;
        }

        if transitions > 0 {
            writeln!(f)?;
        }

        for (from, to, transition) in self.0.transitions() {
            let (from, to) = (from.index(), to.index());
            writeln!(f, "    {} -> {} [label=\"{}\"];", from, to, transition)?;
        }

        write!(f, "}}")
    }
}
