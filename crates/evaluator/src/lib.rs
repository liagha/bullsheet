mod builtins;
mod lexer;
mod parser;
mod registry;
mod syntax;

pub use crate::syntax::Source;

use crate::builtins::{Operator, Operators};
use crate::lexer::{Glyph, Lexer};
use crate::parser::{Parser, Shape, ShapeKind};
use crate::registry::Registry;
use crate::syntax::{Expression, Identifier, Line, Span};
use element::{Arity, Blank, Callable, Failure, Function, Number, Parameter, Text, Value};

#[derive(Clone)]
pub struct Part {
    span: Span,
    value: Value,
}

impl Part {
    pub fn span(&self) -> Span {
        self.span.clone()
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}

pub struct Reading {
    value: Value,
    parts: Vec<Part>,
}

impl Reading {
    fn new(value: Value, parts: Vec<Part>) -> Self {
        Self { value, parts }
    }

    fn without_parts(value: Value) -> Self {
        Self::new(value, Vec::new())
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn parts(&self) -> &[Part] {
        &self.parts
    }
}

pub struct Name {
    text: Text,
    value: Value,
}

impl Name {
    pub fn new(text: Text, value: Value) -> Self {
        Self { text, value }
    }

    pub fn text(&self) -> &Text {
        &self.text
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}

pub struct Readings {
    lines: Vec<Reading>,
    names: Vec<Name>,
}

impl Readings {
    fn new(lines: Vec<Reading>, names: Vec<Name>) -> Self {
        Self { lines, names }
    }

    pub fn names(&self) -> &[Name] {
        &self.names
    }
}

impl std::ops::Deref for Readings {
    type Target = [Reading];

    fn deref(&self) -> &Self::Target {
        &self.lines
    }
}

impl IntoIterator for Readings {
    type Item = Reading;
    type IntoIter = std::vec::IntoIter<Reading>;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.into_iter()
    }
}

impl<'a> IntoIterator for &'a Readings {
    type Item = &'a Reading;
    type IntoIter = std::slice::Iter<'a, Reading>;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.iter()
    }
}

struct Variables {
    values: std::collections::HashMap<Identifier, Value>,
    parent: Option<std::rc::Rc<std::cell::RefCell<Variables>>>,
}

struct Language {
    registry: Registry,
    operators: Operators,
}

impl Language {
    fn new(installed: &[(Identifier, Value)]) -> Self {
        let mut registry = Registry::new();
        registry.add_builtins();
        for (name, value) in installed {
            registry.add(&name.to_string(), value.clone());
        }
        let operators = Operators::new(&registry);
        Self {
            registry,
            operators,
        }
    }
}

struct Session {
    variables: std::rc::Rc<std::cell::RefCell<Variables>>,
    language: std::rc::Rc<Language>,
    depth: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Session {
    fn new(language: std::rc::Rc<Language>) -> Self {
        Self {
            variables: std::rc::Rc::new(std::cell::RefCell::new(Variables {
                values: std::collections::HashMap::new(),
                parent: None,
            })),
            language,
            depth: std::rc::Rc::new(std::cell::Cell::new(0)),
        }
    }

    fn lookup(&self, name: &Identifier) -> Result<Value, Failure> {
        if name.is_answer() {
            return self
                .variable(name)
                .ok_or_else(|| Failure::new("ans is not defined"));
        }
        if let Some(value) = self.variable(name) {
            return Ok(value);
        }
        if let Some(value) = self.language.registry.find(name) {
            return Ok(value.clone());
        }
        Err(Failure::UnknownName {
            name: Text::new(name.to_string()),
        })
    }

    fn variable(&self, name: &Identifier) -> Option<Value> {
        let mut current = Some(std::rc::Rc::clone(&self.variables));
        while let Some(scope) = current {
            let borrow = scope.borrow();
            if let Some(value) = borrow.values.get(name) {
                return Some(value.clone());
            }
            current = borrow.parent.clone();
        }
        None
    }

    fn assign(&self, name: Identifier, value: Value) -> Result<(), Failure> {
        if name.is_answer() || self.language.registry.contains(&name) {
            return Err(Failure::Reserved {
                name: Text::new(name.to_string()),
            });
        }
        self.variables.borrow_mut().values.insert(name, value);
        Ok(())
    }

    fn remember(&self, value: Value) {
        self.variables
            .borrow_mut()
            .values
            .insert(Identifier::new("ans"), value);
    }

    fn names(&self) -> Vec<Name> {
        let mut names = Vec::new();
        let mut current = Some(std::rc::Rc::clone(&self.variables));
        while let Some(scope) = current {
            let borrow = scope.borrow();
            for (identifier, value) in &borrow.values {
                if identifier.is_answer() {
                    continue;
                }
                names.push(Name {
                    text: Text::new(identifier.to_string()),
                    value: value.clone(),
                });
            }
            current = borrow.parent.clone();
        }
        names.sort_by(|left, right| left.text.as_ref().cmp(right.text.as_ref()));
        names.extend(self.language.registry.names());
        names.extend(self.key_names());
        names
    }

    fn key_names(&self) -> Vec<Name> {
        let mut names: Vec<Name> = [
            (element::Modifiers::CONTROL, "Ctrl"),
            (element::Modifiers::ALT, "Alt"),
            (element::Modifiers::SHIFT, "Shift"),
            (element::Modifiers::SUPER, "Super"),
        ]
        .into_iter()
        .map(|(modifiers, text)| Name {
            text: Text::new(text),
            value: Value::new(element::Key::modifier(modifiers)),
        })
        .collect();
        for text in [
            "Space", "Enter", "Escape", "Tab", "Backspace", "Delete", "Insert", "Home", "End",
            "PageUp", "PageDown", "Left", "Right", "Up", "Down",
        ] {
            names.push(Name {
                text: Text::new(text),
                value: Value::new(element::Key::new(element::Modifiers::NONE, Text::new(text))),
            });
        }
        for number in 1..=24 {
            names.push(Self::function_key(number));
        }
        for letter in b'A'..=b'Z' {
            let text = (letter as char).to_string();
            names.push(Name {
                text: Text::new(text.as_str()),
                value: Value::new(element::Key::new(
                    element::Modifiers::NONE,
                    Text::new(text.as_str()),
                )),
            });
        }
        names
    }

    fn function_key(number: u8) -> Name {
        let text = format!("F{number}");
        Name {
            text: Text::new(text.as_str()),
            value: Value::new(element::Key::new(
                element::Modifiers::NONE,
                Text::new(text.as_str()),
            )),
        }
    }
}

struct Symbol {
    name: Identifier,
    parameters: Vec<Identifier>,
    body: Shape,
    scope: std::rc::Weak<std::cell::RefCell<Variables>>,
    language: std::rc::Rc<Language>,
    depth: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Symbol {
    fn evaluate(&self, values: &[Value]) -> Result<Value, Failure> {
        let scope = self
            .scope
            .upgrade()
            .ok_or_else(|| Failure::new("the definition is out of scope"))?;
        let mut bound = std::collections::HashMap::new();
        for (parameter, value) in self.parameters.iter().zip(values.iter()) {
            bound.insert(parameter.clone(), value.clone());
        }
        let session = Session {
            variables: std::rc::Rc::new(std::cell::RefCell::new(Variables {
                values: bound,
                parent: Some(scope),
            })),
            language: std::rc::Rc::clone(&self.language),
            depth: std::rc::Rc::clone(&self.depth),
        };
        Evaluator::new(&session).value(&self.body)
    }
}

impl Callable for Symbol {
    fn name(&self) -> Text {
        Text::new(self.name.to_string())
    }

    fn arity(&self) -> Arity {
        Arity::exactly(self.parameters.len())
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let depth = self.depth.get() + 1;
        if depth > 64 {
            return Err(Failure::TooDeep);
        }
        self.depth.set(depth);
        let result = self.evaluate(values);
        self.depth.set(depth - 1);
        result
    }
}

struct Thunk {
    shape: std::rc::Rc<Shape>,
    variables: std::rc::Rc<std::cell::RefCell<Variables>>,
    language: std::rc::Rc<Language>,
    depth: std::rc::Rc<std::cell::Cell<usize>>,
    parts: Option<std::rc::Rc<std::cell::RefCell<Vec<Part>>>>,
}

impl Callable for Thunk {
    fn name(&self) -> Text {
        Text::new("expression")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(0)
    }

    fn call(&self, _values: &[Value]) -> Result<Value, Failure> {
        let session = Session {
            variables: std::rc::Rc::clone(&self.variables),
            language: std::rc::Rc::clone(&self.language),
            depth: std::rc::Rc::clone(&self.depth),
        };
        match &self.parts {
            Some(parts) => Evaluator::build(&session, std::rc::Rc::clone(parts)).value(&self.shape),
            None => Evaluator::new(&session).value(&self.shape),
        }
    }
}

struct Evaluator<'a> {
    session: &'a Session,
    parts: Option<std::rc::Rc<std::cell::RefCell<Vec<Part>>>>,
}

impl<'a> Evaluator<'a> {
    fn new(session: &'a Session) -> Self {
        Self {
            session,
            parts: None,
        }
    }

    fn build(session: &'a Session, parts: std::rc::Rc<std::cell::RefCell<Vec<Part>>>) -> Self {
        Self {
            session,
            parts: Some(parts),
        }
    }

    fn value(&mut self, shape: &Shape) -> Result<Value, Failure> {
        let result = self.evaluate(shape);
        if Self::is_leaf(shape) {
            let recorded = match &result {
                Ok(value) => Some(value.clone()),
                Err(failure) if matches!(shape.kind(), ShapeKind::Name(_)) => {
                    Some(Value::new(failure.clone()))
                }
                Err(_) => None,
            };
            if let Some(value) = recorded {
                self.record(shape, value);
            }
        }
        result
    }

    fn is_leaf(shape: &Shape) -> bool {
        matches!(
            shape.kind(),
            ShapeKind::Number(_)
                | ShapeKind::Text(_)
                | ShapeKind::Key(_)
                | ShapeKind::Name(_)
                | ShapeKind::Glyph(_)
        )
    }

    fn evaluate(&mut self, shape: &Shape) -> Result<Value, Failure> {
        match shape.kind() {
            ShapeKind::Number(number) => Ok(Value::new(*number)),
            ShapeKind::Text(text) => Ok(Value::new(text.clone())),
            ShapeKind::Key(key) => Ok(Value::new(key.clone())),
            ShapeKind::Name(name) => self.session.lookup(name),
            ShapeKind::Glyph(glyph) => self
                .session
                .language
                .operators
                .value(glyph)
                .ok_or_else(|| Failure::new(format!("unexpected symbol {glyph}"))),
            ShapeKind::Unary { operator, operand } => {
                let function = self.operator(operator, Operators::prefix)?;
                let value = self.value(operand)?;
                function.call(&[value])
            }
            ShapeKind::Binary {
                operator,
                left,
                right,
            } => {
                let left = self.value(left)?;
                let function = self.operator(operator, Operators::infix)?;
                let right = self.value(right)?;
                function.call(&[left, right])
            }
            ShapeKind::Postfix { operator, operand } => {
                let value = self.value(operand)?;
                let function = self.operator(operator, Operators::postfix)?;
                function.call(&[value])
            }
            ShapeKind::Multiplication {
                operator,
                left,
                right,
            } => {
                let function = self
                    .session
                    .language
                    .operators
                    .infix(operator)
                    .map(Operator::function)
                    .ok_or_else(|| Failure::new(format!("unexpected symbol {operator}")))?;
                let left = self.value(left)?;
                let right = self.value(right)?;
                function.call(&[left, right])
            }
            ShapeKind::Binding { target, value } => self.binding(target, value),
            ShapeKind::Call {
                function,
                arguments,
            } => {
                let name = match function.kind() {
                    ShapeKind::Name(name) => name.to_string(),
                    _ => String::from("expression"),
                };
                let value = self.value(function)?;
                let callable = value
                    .cast::<Function>()
                    .cloned()
                    .ok_or_else(|| Failure::new(format!("{name} is not a function")))?;
                let values = if callable.lazy() {
                    arguments
                        .iter()
                        .map(|argument| Value::new(Function::new(self.thunk(argument))))
                        .collect()
                } else {
                    let mut values = Vec::with_capacity(arguments.len());
                    for argument in arguments {
                        values.push(self.value(argument)?);
                    }
                    values
                };
                callable.call(&values)
            }
        }
    }

    fn binding(&mut self, target: &Shape, value: &Shape) -> Result<Value, Failure> {
        match target.kind() {
            ShapeKind::Name(name) => {
                let value = self.value(value)?;
                self.session.assign(name.clone(), value.clone())?;
                self.record(target, value.clone());
                Ok(value)
            }
            ShapeKind::Call {
                function,
                arguments,
            } => {
                let name = match function.kind() {
                    ShapeKind::Name(name) => name.clone(),
                    _ => return Err(Failure::new("a binding needs a name")),
                };
                let mut parameters = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    match argument.kind() {
                        ShapeKind::Name(parameter) => {
                            parameters.push((parameter.clone(), argument.clone()));
                        }
                        _ => return Err(Failure::new("a parameter must be a name")),
                    }
                }
                let bound = Value::new(Function::new(Symbol {
                    name: name.clone(),
                    parameters: parameters
                        .iter()
                        .map(|(parameter, _)| parameter.clone())
                        .collect(),
                    body: value.clone(),
                    scope: std::rc::Rc::downgrade(&self.session.variables),
                    language: std::rc::Rc::clone(&self.session.language),
                    depth: std::rc::Rc::clone(&self.session.depth),
                }));
                self.session.assign(name, bound.clone())?;
                self.record(function, bound.clone());
                for (parameter, shape) in parameters {
                    self.record(
                        &shape,
                        Value::new(Parameter::new(Text::new(parameter.to_string()))),
                    );
                }
                Ok(bound)
            }
            _ => Err(Failure::new("a binding needs a name")),
        }
    }

    fn operator(
        &mut self,
        shape: &Shape,
        select: for<'b> fn(&'b Operators, &Glyph) -> Option<&'b Operator>,
    ) -> Result<Function, Failure> {
        let glyph = match shape.kind() {
            ShapeKind::Glyph(glyph) => glyph,
            _ => return Err(Failure::new("unexpected operator")),
        };
        let function = select(&self.session.language.operators, &glyph)
            .map(Operator::function)
            .ok_or_else(|| Failure::new(format!("unexpected symbol {glyph}")))?;
        self.record(shape, Value::new(function.clone()));
        Ok(function)
    }

    fn record(&mut self, shape: &Shape, value: Value) {
        if let Some(parts) = &self.parts {
            parts.borrow_mut().push(Part {
                span: shape.span(),
                value,
            });
        }
    }

    fn thunk(&self, shape: &Shape) -> Thunk {
        Thunk {
            shape: std::rc::Rc::new(shape.clone()),
            variables: std::rc::Rc::clone(&self.session.variables),
            language: std::rc::Rc::clone(&self.session.language),
            depth: std::rc::Rc::clone(&self.session.depth),
            parts: self.parts.clone(),
        }
    }
}

pub struct Calculator {
    installed: Vec<(Identifier, Value)>,
    language: std::rc::Rc<Language>,
}

impl Calculator {
    pub fn new() -> Self {
        let installed = Vec::new();
        let language = std::rc::Rc::new(Language::new(&installed));
        Self {
            installed,
            language,
        }
    }

    pub fn install(&mut self, name: &str, function: Function) {
        self.installed
            .push((Identifier::new(name), Value::new(function)));
        self.language = std::rc::Rc::new(Language::new(&self.installed));
    }

    pub fn evaluate(&self, source: &Source) -> Readings {
        let session = Session::new(std::rc::Rc::clone(&self.language));
        let mut readings = Vec::with_capacity(source.line_count());
        readings.extend(
            source
                .lines()
                .map(|line| self.evaluate_line(line, &session)),
        );
        Readings::new(readings, session.names())
    }

    fn evaluate_line(&self, line: Line<'_>, session: &Session) -> Reading {
        let expression = line.expression();
        if expression.is_empty() {
            return Reading::without_parts(Value::new(Blank::new()));
        }
        let reading = self.evaluate_expression(&expression, session);
        if reading.value().cast::<Failure>().is_none() {
            session.remember(reading.value().clone());
        }
        reading
    }

    fn parse(&self, expression: &Expression, session: &Session) -> Result<Shape, Failure> {
        let mut lexer = Lexer::new(expression);
        let tokens = lexer.tokens()?;
        Parser::new(tokens, &session.language.operators, expression).shape()
    }

    fn evaluate_expression(&self, expression: &Expression, session: &Session) -> Reading {
        let parts = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let result = self
            .parse(expression, session)
            .and_then(|shape| Evaluator::build(session, std::rc::Rc::clone(&parts)).value(&shape));
        let value = match result {
            Ok(value)
                if value
                    .cast::<Number>()
                    .is_some_and(|number| number.value().is_nan()) =>
            {
                Value::new(Failure::new("not a real number"))
            }
            Ok(value) => value,
            Err(failure) => Value::new(failure),
        };
        let parts = std::rc::Rc::try_unwrap(parts).map_or_else(
            |shared| shared.borrow().clone(),
            std::cell::RefCell::into_inner,
        );
        Reading::new(value, parts)
    }
}
