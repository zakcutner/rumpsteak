use super::Fsm;
use std::fmt::{self, Display, Formatter};

pub struct Petrify<'a, R, L>(&'a Fsm<R, L>);

impl<'a, R, L> Petrify<'a, R, L> {
    pub fn new(fsm: &'a Fsm<R, L>) -> Self {
        assert!(fsm.size().0 > 0);
        Self(fsm)
    }
}

impl<'a, R: Display, L: Display> Display for Petrify<'a, R, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, ".outputs")?;
        writeln!(f, ".state graph")?;

        for (from, to, transition) in self.0.transitions() {
            let (from, to) = (from.index(), to.index());
            let (role, action, label) = (transition.role, transition.action, transition.label);
            writeln!(f, "s{} {} {} {} s{}", from, role, action, label, to)?;
        }

        writeln!(f, ".marking s0")?;
        write!(f, ".end")
    }
}
