use crate::syntax::Identifier;
use crate::Name;
use element::{Callable, Function, Text, Value};
use std::collections::HashMap;

/// Every name the sheet knows, in one place.
pub struct Registry {
    values: HashMap<Identifier, Value>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: &str, value: Value) {
        self.values.insert(Identifier::new(name), value);
    }

    pub fn add_callable(&mut self, name: &str, callable: impl Callable) {
        self.add(name, Value::new(Function::new(callable)));
    }

    pub fn find(&self, name: &Identifier) -> Option<&Value> {
        self.values.get(name)
    }

    pub fn function(&self, name: &str) -> Function {
        self.find(&Identifier::new(name))
            .and_then(|value| value.cast::<Function>())
            .cloned()
            .expect("every operator names a function the registry holds")
    }

    pub fn contains(&self, name: &Identifier) -> bool {
        self.values.contains_key(name)
    }

    pub fn names(&self) -> Vec<Name> {
        let mut names: Vec<Name> = self
            .values
            .iter()
            .map(|(identifier, value)| Name::new(Text::new(identifier.to_string()), value.clone()))
            .collect();
        names.sort_by(|left, right| left.text().as_ref().cmp(right.text().as_ref()));
        names
    }
}
