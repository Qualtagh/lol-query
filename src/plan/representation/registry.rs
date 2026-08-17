//! Caller-registered opaque Apply / Fold monoids and runtime scalar values.
//!
//! Bodies live here; the frozen graph stores only ids.

#![allow(dead_code)]

use std::rc::Rc;

use super::expr::Literal;
use super::id::{ApplyId, MonoidId, NodeId, ParamId};

/// Runtime scalar / bag value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Unit,
    /// Missing attribute (or other nullish projection).
    Null,
    Bool(bool),
    Int(i64),
    Str(Rc<str>),
    Node(NodeId),
    List(Vec<Value>),
}

impl Value {
    pub(crate) fn str(s: impl AsRef<str>) -> Self {
        Self::Str(Rc::from(s.as_ref()))
    }

    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    pub(crate) fn as_list(&self) -> Option<&[Value]> {
        match self {
            Self::List(xs) => Some(xs),
            _ => None,
        }
    }
}

impl From<Literal> for Value {
    fn from(value: Literal) -> Self {
        match value {
            Literal::Unit => Self::Unit,
            Literal::Bool(b) => Self::Bool(b),
            Literal::Int(n) => Self::Int(n),
            Literal::Str(s) => Self::Str(s),
        }
    }
}

struct Monoid {
    identity: Value,
    append: Box<dyn Fn(Value, Value) -> Value>,
}

/// Caller-registered Apply / Fold monoid / parameter slots for a plan.
pub(crate) struct Registry {
    applies: Vec<Box<dyn Fn(&[Value]) -> Value>>,
    monoids: Vec<Monoid>,
    params: Vec<Value>,
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self { applies: Vec::new(), monoids: Vec::new(), params: Vec::new() }
    }

    pub(crate) fn register_apply(&mut self, f: impl Fn(&[Value]) -> Value + 'static) -> ApplyId {
        let id = ApplyId::new(self.applies.len() as u64);
        self.applies.push(Box::new(f));
        id
    }

    pub(crate) fn register_monoid(&mut self, identity: Value, append: impl Fn(Value, Value) -> Value + 'static) -> MonoidId {
        let id = MonoidId::new(self.monoids.len() as u64);
        self.monoids.push(Monoid { identity, append: Box::new(append) });
        id
    }

    pub(crate) fn set_param(&mut self, id: ParamId, value: Value) {
        let i = id.raw() as usize;
        if self.params.len() <= i {
            self.params.resize(i + 1, Value::Unit);
        }
        self.params[i] = value;
    }

    pub(crate) fn apply(&self, id: ApplyId, args: &[Value]) -> Value {
        self.applies.get(id.raw() as usize).expect("unregistered ApplyId")(args)
    }

    pub(crate) fn monoid_identity(&self, id: MonoidId) -> Value {
        self.monoid(id).identity.clone()
    }

    pub(crate) fn monoid_append(&self, id: MonoidId, acc: Value, item: Value) -> Value {
        (self.monoid(id).append)(acc, item)
    }

    pub(crate) fn param(&self, id: ParamId) -> &Value {
        self.params.get(id.raw() as usize).expect("unset ParamId")
    }

    fn monoid(&self, id: MonoidId) -> &Monoid {
        self.monoids.get(id.raw() as usize).expect("unregistered MonoidId")
    }
}
