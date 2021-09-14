use super::Fsm;
use std::fmt::{self, Display, Formatter};

pub struct Petrify<'a, R, N, E>(&'a Fsm<R, N, E>);

impl<'a, R, N, E> Petrify<'a, R, N, E> {
    pub fn new(fsm: &'a Fsm<R, N, E>) -> Self {
        assert!(fsm.size().0 > 0);
        Self(fsm)
    }
}

impl<'a, R: Display, N: Display, E> Display for Petrify<'a, R, N, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, ".outputs")?;
        writeln!(f, ".state graph")?;

        for (from, to, transition) in self.0.transitions() {
            let (from, to) = (from.index(), to.index());
            let (role, action, message) = (transition.role, transition.action, transition.message);
            writeln!(f, "s{} {} {} {} s{}", from, role, action, message.label, to)?;
        }

        writeln!(f, ".marking s0")?;
        write!(f, ".end")
    }
}
