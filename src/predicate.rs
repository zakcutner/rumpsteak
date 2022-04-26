use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::marker::PhantomData;

use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::cmp::Ord;
use std::hash::Hash;

/// A `Predicate` is a structure that allows to express some properties on some
/// variables. The trait contains a single function which assigns a truth value
/// to a set of variables.
/// A Predicate must implement the `Default` so that it can be instantiated by
/// the system.
/// [min_const_generics](https://github.com/rust-lang/rust/issues/74878) help to
/// implement such types in our context.
pub trait Predicate: Default {
    type Name;
    type Value;
    type Label;
    type Error: Debug;

    /// This function checks whether the predicate holds on the current values
    /// of variables.
    /// It returns a `Result` which is `Ok(())` if the predicate holds, or an
    /// arbitrary error (of type `E`) if not.
    /// Most likely, one may want to have `()` as the error type (falling back
    /// on something simili-boolean), but having this generic type allow more
    /// precise analysis in case of failure (some kind of very basic causal
    /// analysis).
    ///
    /// The default implementation always returns `Ok(())`.
    fn check(
        &self,
        _m: &HashMap<Self::Name, Self::Value>,
        _label: Option<&Self::Label>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// The `Tautology` struct implements a tautology predicate (i.e. always valid).
pub struct Tautology<N, V, L> {
    _ph: PhantomData<(N, V, L)>,
}

impl<N, V, L> Default for Tautology<N, V, L> {
    fn default() -> Self {
        Self { _ph: PhantomData }
    }
}

impl<N, V, L> Predicate for Tautology<N, V, L> {
    type Name = N;
    type Value = V;
    type Label = L;

    type Error = ();
}

pub struct LTnVar<V: Ord, L, const LHS: char, const RHS: char> {
    _p: PhantomData<(V, L)>,
}

impl<L, V: Ord, const LHS: char, const RHS: char> Default for LTnVar<V, L, LHS, RHS> {
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<L, V: Ord, const LHS: char, const RHS: char> Predicate for LTnVar<V, L, LHS, RHS>
where
    V: std::cmp::Ord,
{
    type Name = char;
    type Value = V;
    type Label = L;
    type Error = ();

    fn check(
        &self,
        m: &HashMap<Self::Name, Self::Value>,
        _l: Option<&Self::Label>,
    ) -> Result<(), Self::Error> {
        let lhs = m.get(&LHS).ok_or(())?;
        let rhs = m.get(&RHS).ok_or(())?;
        if lhs < rhs {
            Ok(())
        } else {
            Err(())
        }
    }
}

pub struct LTnConst<L, const LHS: char, const RHS: i32> {
    _p: PhantomData<L>,
}

impl<L, const LHS: char, const RHS: i32> Default for LTnConst<L, LHS, RHS> {
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<L, const LHS: char, const RHS: i32> Predicate for LTnConst<L, LHS, RHS> {
    type Name = char;
    type Value = i32;
    type Label = L;
    type Error = ();

    fn check(
        &self,
        m: &HashMap<Self::Name, Self::Value>,
        _l: Option<&Self::Label>,
    ) -> Result<(), Self::Error> {
        let lhs: Self::Value = *m.get(&LHS).ok_or(())?;
        if lhs < RHS {
            Ok(())
        } else {
            Err(())
        }
    }
}

pub struct LTn<N, V> {
    lhs: N,
    rhs: N,
    _p: PhantomData<V>,
}

impl<N: Eq + Hash, V: PartialOrd> LTn<N, V> {
    fn new(lhs: N, rhs: N) -> Self {
        Self {
            lhs,
            rhs,
            _p: PhantomData,
        }
    }

    fn check(&self, m: &HashMap<N, V>) -> Result<(), ()> {
        let lhs = m.get(&self.lhs).ok_or(())?;
        let rhs = m.get(&self.rhs).ok_or(())?;
        if lhs < rhs {
            Ok(())
        } else {
            Err(())
        }
    }
}

#[macro_export]
macro_rules! formula {
    ( $lhs:ident < $rhs:ident ) => {
        LTn::new($lhs, $rhs)
    };
}

pub enum Formula<N, V> {
    LTn(LTn<N, V>),
}

#[derive(Parser)]
#[grammar = "parser/predicate.pest"]
struct PredicateParser;

impl<'a, N, V> From<Pair<'a, Rule>> for LTn<N, V>
where
    N: From<&'a str> + std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Ord,
{
    fn from(p: Pair<'a, Rule>) -> LTn<N, V> {
        let mut pairs = p.into_inner();
        let lhs: N = pairs.next().unwrap().as_str().into();
        let rhs: N = pairs.next().unwrap().as_str().into();

        LTn::new(lhs, rhs)
    }
}

impl<'a, N, V> TryFrom<&'a str> for Formula<N, V>
where
    N: From<&'a str> + std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Ord,
{
    type Error = Error<Rule>;

    fn try_from(s: &'a str) -> Result<Formula<N, V>, Error<Rule>> {
        let mut pairs = PredicateParser::parse(Rule::ltn, s)?;
        let pair = pairs.next().unwrap();
        let formula = match pair.as_rule() {
            Rule::ltn => Formula::LTn(pair.into()),
            _ => panic!(),
        };
        Ok(formula)
    }
}
