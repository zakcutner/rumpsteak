use std::marker::PhantomData;
use std::collections::HashMap;

/// A `SideEffect` is a structure that allows arbitrary modifications of a set
/// of variables.
pub trait SideEffect: Default {
    type Name;
    type Value;

    /// This function modifies the values of variables.
    ///
    /// The default implementation does not modify the variables.
    fn side_effect(&mut self, _m: &mut HashMap<Self::Name, Self::Value>) {}
}

/// The `Constant` struct implements a side effect that does nothing.
pub struct Constant<N, V> {
    _ph: PhantomData<(N, V)>,
}

impl<N, V> Default for Constant<N, V> {
    fn default() -> Self {
        Self { _ph: PhantomData }
    }
}

impl<N, V> SideEffect for Constant<N, V> {
    type Name = N;
    type Value = V;
}
