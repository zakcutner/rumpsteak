use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;

use std::cmp::Ord;

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

/// The `LTnVar` struct implements an operator (less than a variable).
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

/// The `LTnConst` struct implements an operator (less than a value).
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

/// The `GTnVar` struct implements an operator (greater than a variable).
pub struct GTnVar<V: Ord, L, const LHS: char, const RHS: char> {
    _p: PhantomData<(V, L)>,
}

impl<L, V: Ord, const LHS: char, const RHS: char> Default for GTnVar<V, L, LHS, RHS> {
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<L, V: Ord, const LHS: char, const RHS: char> Predicate for GTnVar<V, L, LHS, RHS>
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
        if lhs > rhs {
            Ok(())
        } else {
            Err(())
        }
    }
}

/// The `GTnConst` struct implements an operator (less than a value).
pub struct GTnConst<L, const LHS: char, const RHS: i32> {
    _p: PhantomData<L>,
}

impl<L, const LHS: char, const RHS: i32> Default for GTnConst<L, LHS, RHS> {
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<L, const LHS: char, const RHS: i32> Predicate for GTnConst<L, LHS, RHS> {
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
        if lhs > RHS {
            Ok(())
        } else {
            Err(())
        }
    }
}

/// The `EqualVar` struct implements an operator (equal to a variable).
pub struct EqualVar<V: Ord, L, const LHS: char, const RHS: char> {
    _p: PhantomData<(V, L)>,
}

impl<L, V: Ord, const LHS: char, const RHS: char> Default for EqualVar<V, L, LHS, RHS> {
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<L, V: Ord, const LHS: char, const RHS: char> Predicate for EqualVar<V, L, LHS, RHS>
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
        if lhs == rhs {
            Ok(())
        } else {
            Err(())
        }
    }
}

/// The `EqualConst` struct implements an operator (equal to a value).
pub struct EqualConst<L, const LHS: char, const RHS: i32> {
    _p: PhantomData<L>,
}

impl<L, const LHS: char, const RHS: i32> Default for EqualConst<L, LHS, RHS> {
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<L, const LHS: char, const RHS: i32> Predicate for EqualConst<L, LHS, RHS> {
    type Name = char;
    type Value = i32;
    type Label = L;
    type Error = ();

    fn check(
        &self,
        m: &HashMap<Self::Name, Self::Value>,
        _l: Option<&Self::Label>,
    ) -> Result<(), Self::Error> {
        let lhs = m.get(&LHS).ok_or(())?;
        if lhs == &RHS {
            Ok(())
        } else {
            Err(())
        }
    }
}

/// The `LTnThree` struct implements an operator (less than between three operands).
pub struct LTnThree<V: Ord, L, const LHS: char, const MID: char, const RHS: char> {
    _p: PhantomData<(V, L)>,
}

impl<L, V: Ord, const LHS: char, const MID: char, const RHS: char> Default
    for LTnThree<V, L, LHS, MID, RHS>
{
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<L, V: Ord, const LHS: char, const MID: char, const RHS: char> Predicate
    for LTnThree<V, L, LHS, MID, RHS>
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
        let mid = m.get(&MID).ok_or(())?;
        let rhs = m.get(&RHS).ok_or(())?;
        if lhs < mid && mid < rhs {
            Ok(())
        } else {
            Err(())
        }
    }
}

/// The `GTnThree` struct implements an operator (greater than between three operands).
pub struct GTnThree<V: Ord, L, const LHS: char, const MID: char, const RHS: char> {
    _p: PhantomData<(V, L)>,
}

impl<L, V: Ord, const LHS: char, const MID: char, const RHS: char> Default
    for GTnThree<V, L, LHS, MID, RHS>
{
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<L, V: Ord, const LHS: char, const MID: char, const RHS: char> Predicate
    for GTnThree<V, L, LHS, MID, RHS>
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
        let mid = m.get(&MID).ok_or(())?;
        let rhs = m.get(&RHS).ok_or(())?;
        if lhs > mid && mid > rhs {
            Ok(())
        } else {
            Err(())
        }
    }
}

/// The `And` struct implements an and operator (e.g. pred1 and pred 2).
pub struct And<LHS: Predicate, RHS: Predicate> {
    _p: PhantomData<(LHS, RHS)>,
}

impl<
        LHS: Predicate,
        RHS: Predicate,
    > Default for And<LHS, RHS>
{
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<
        L,
        LHS: Predicate<Name = N, Value = V, Label = L, Error = ()>,
        RHS: Predicate<Name = N, Value = V, Label = L, Error = ()>,
        N,
        V,
    > Predicate for And<LHS, RHS>
{
    type Name = N;
    type Value = V;
    type Label = L;
    type Error = ();

    fn check(
        &self,
        m: &HashMap<Self::Name, Self::Value>,
        _l: Option<&Self::Label>,
    ) -> Result<(), Self::Error> {
        if LHS::default().check(m, None).is_ok() && RHS::default().check(m, None).is_ok() {
            Ok(())
        } else {
            Err(())
        }
    }
}

pub struct Or<L, LHS: Predicate, RHS: Predicate, N, V> {
    _p: PhantomData<(LHS, RHS, N, V, L)>,
}

impl<
        L,
        LHS: Predicate<Name = N, Value = V, Error = ()>,
        RHS: Predicate<Name = N, Value = V, Error = ()>,
        N,
        V,
    > Default for Or<L, LHS, RHS, N, V>
{
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<
        L,
        LHS: Predicate<Name = N, Value = V, Error = ()>,
        RHS: Predicate<Name = N, Value = V, Error = ()>,
        N,
        V,
    > Predicate for Or<L, LHS, RHS, N, V>
{
    type Name = N;
    type Value = V;
    type Label = L;
    type Error = ();

    fn check(
        &self,
        m: &HashMap<Self::Name, Self::Value>,
        _l: Option<&Self::Label>,
    ) -> Result<(), Self::Error> {
        if LHS::default().check(m, None).is_ok() || RHS::default().check(m, None).is_ok() {
            Ok(())
        } else {
            Err(())
        }
    }
}

pub struct Neg<L, LHS: Predicate, N, V> {
    _p: PhantomData<(LHS, N, V, L)>,
}

impl<L, LHS: Predicate<Name = N, Value = V, Error = ()>, N, V> Default for Neg<L, LHS, N, V> {
    fn default() -> Self {
        Self { _p: PhantomData }
    }
}

impl<L, LHS: Predicate<Name = N, Value = V, Error = ()>, N, V> Predicate for Neg<L, LHS, N, V> {
    type Name = N;
    type Value = V;
    type Label = L;
    type Error = ();

    fn check(
        &self,
        m: &HashMap<Self::Name, Self::Value>,
        _l: Option<&Self::Label>,
    ) -> Result<(), Self::Error> {
        if LHS::default().check(m, None).is_ok() {
            Err(())
        } else {
            Ok(())
        }
    }
}
