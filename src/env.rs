//! Lexical environments (scopes) for the Azra VM.
//!
//! Scopes form a parent chain: reads walk outward, `set` updates the nearest
//! existing binding (so functions can adjust outer variables) and new names
//! are defined in the current scope.

use crate::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A single lexical scope.
#[derive(Debug, Default)]
pub struct Scope {
    vars: RefCell<HashMap<String, Value>>,
    parent: Option<Rc<Scope>>,
}

impl Scope {
    /// A fresh global/root scope (no parent).
    pub fn root() -> Rc<Scope> {
        Rc::new(Scope::default())
    }

    /// A child scope nested inside `parent`.
    pub fn new(parent: &Rc<Scope>) -> Rc<Scope> {
        Rc::new(Scope {
            vars: RefCell::new(HashMap::new()),
            parent: Some(parent.clone()),
        })
    }

    /// Look up a binding, walking outward through enclosing scopes.
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.vars.borrow().get(name) {
            return Some(v.clone());
        }
        self.parent.as_ref().and_then(|p| p.get(name))
    }

    /// Update the nearest existing binding; returns `false` if none exists.
    pub fn set_existing(&self, name: &str, value: Value) -> bool {
        let mut cur = Some(self);
        while let Some(scope) = cur {
            {
                let mut vars = scope.vars.borrow_mut();
                if let Some(slot) = vars.get_mut(name) {
                    *slot = value;
                    return true;
                }
            }
            cur = scope.parent.as_deref();
        }
        false
    }

    /// Bind a name in this scope only.
    pub fn define(&self, name: &str, value: Value) {
        self.vars.borrow_mut().insert(name.to_string(), value);
    }

    /// All bindings defined directly in this scope (used for `use` exports).
    pub fn entries(&self) -> Vec<(String, Value)> {
        self.vars
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}
