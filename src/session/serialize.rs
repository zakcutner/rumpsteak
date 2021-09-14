#![cfg(feature = "serialize")]

use crate::{
    fsm::{Action, Fsm, Message, StateIndex, Transition},
    Branch, End, FromState, Receive, Role, Select, Send,
};
use std::{
    any::{type_name, TypeId},
    collections::{hash_map::Entry, HashMap},
    convert::Infallible,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
};

#[derive(Clone, Copy, Debug, Eq)]
pub struct Type {
    id: TypeId,
    name: &'static str,
}

impl Type {
    fn new<T: 'static>() -> Self {
        Self {
            id: TypeId::of::<T>(),
            name: type_name::<T>(),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct Serializer {
    fsm: Fsm<Type, Type, Infallible>,
    history: HashMap<TypeId, StateIndex>,
    previous: Option<(StateIndex, Transition<Type, Type, Infallible>)>,
}

impl Serializer {
    fn add_state_index(&mut self, state: StateIndex) {
        if let Some((previous, transition)) = self.previous.take() {
            self.fsm
                .add_transition(previous, state, transition)
                .unwrap();
        }
    }

    fn add_state<S: 'static>(&mut self) -> Option<StateIndex> {
        match self.history.entry(TypeId::of::<S>()) {
            Entry::Occupied(entry) => {
                let state = *entry.get();
                self.add_state_index(state);
                None
            }
            Entry::Vacant(entry) => {
                let state = self.fsm.add_state();
                entry.insert(state);
                self.add_state_index(state);
                Some(state)
            }
        }
    }

    fn serialize_end<S: 'static>(&mut self) {
        self.add_state::<S>();
    }

    fn serialize_choices<S: 'static, R: 'static>(
        &mut self,
        action: Action,
    ) -> Option<ChoicesSerializer> {
        self.add_state::<S>().map(move |state| ChoicesSerializer {
            serializer: self,
            state,
            role: Type::new::<R>(),
            action,
        })
    }
}

pub struct ChoicesSerializer<'a> {
    serializer: &'a mut Serializer,
    state: StateIndex,
    role: Type,
    action: Action,
}

impl ChoicesSerializer<'_> {
    pub fn serialize_choice<L: 'static, S: Serialize>(&mut self) {
        let message = Message::from_label(Type::new::<L>());
        let transition = Transition::new(self.role, self.action, message);
        self.serializer.previous = Some((self.state, transition));
        S::serialize(&mut self.serializer);
    }
}

pub trait Serialize: 'static {
    fn serialize(s: &mut Serializer);
}

pub trait SerializeChoices: 'static {
    fn serialize_choices(s: ChoicesSerializer<'_>);
}

impl<R: Role + 'static> Serialize for End<'static, R> {
    fn serialize(s: &mut Serializer) {
        s.serialize_end::<Self>();
    }
}

impl<Q: Role + 'static, R: 'static, L: 'static, S> Serialize for Send<'static, Q, R, L, S>
where
    S: FromState<'static, Role = Q> + Serialize,
{
    fn serialize(s: &mut Serializer) {
        if let Some(mut s) = s.serialize_choices::<Self, R>(Action::Output) {
            s.serialize_choice::<L, S>();
        }
    }
}

impl<Q: Role + 'static, R: 'static, L: 'static, S> Serialize for Receive<'static, Q, R, L, S>
where
    S: FromState<'static, Role = Q> + Serialize,
{
    fn serialize(s: &mut Serializer) {
        if let Some(mut s) = s.serialize_choices::<Self, R>(Action::Input) {
            s.serialize_choice::<L, S>();
        }
    }
}

impl<Q: Role + 'static, R: 'static, C: SerializeChoices> Serialize for Select<'static, Q, R, C> {
    fn serialize(s: &mut Serializer) {
        if let Some(s) = s.serialize_choices::<Self, R>(Action::Output) {
            C::serialize_choices(s);
        }
    }
}

impl<Q: Role + 'static, R: 'static, C: SerializeChoices> Serialize for Branch<'static, Q, R, C> {
    fn serialize(s: &mut Serializer) {
        if let Some(s) = s.serialize_choices::<Self, R>(Action::Input) {
            C::serialize_choices(s);
        }
    }
}

pub fn serialize<S: FromState<'static> + Serialize>() -> Fsm<Type, Type, Infallible> {
    let mut serializer = Serializer {
        fsm: Fsm::new(Type::new::<S::Role>()),
        history: HashMap::new(),
        previous: None,
    };

    S::serialize(&mut serializer);
    serializer.fsm
}
