use element::{Callable, Failure, Function, Text, Truth, Value};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct Settings {
    combination: Rc<RefCell<crate::shortcut::Combination>>,
    shortcut: Rc<RefCell<Option<crate::shortcut::Shortcut>>>,
    autostart: Option<Rc<crate::autostart::Autostart>>,
    update: Rc<RefCell<Option<String>>>,
}

impl Settings {
    pub(crate) fn new(
        combination: crate::shortcut::Combination,
        shortcut: Option<crate::shortcut::Shortcut>,
        autostart: Option<crate::autostart::Autostart>,
    ) -> Self {
        Self {
            combination: Rc::new(RefCell::new(combination)),
            shortcut: Rc::new(RefCell::new(shortcut)),
            autostart: autostart.map(Rc::new),
            update: Rc::new(RefCell::new(None)),
        }
    }

    pub(crate) fn install(
        &self,
        calculator: &mut evaluator::Calculator,
        context: &eframe::egui::Context,
    ) {
        calculator.install(
            "shortcut",
            Function::new(ShortcutSetting {
                context: context.clone(),
                combination: Rc::clone(&self.combination),
                shortcut: Rc::clone(&self.shortcut),
            }),
        );
        calculator.install(
            "autostart",
            Function::new(AutostartSetting {
                autostart: self.autostart.clone(),
            }),
        );
        calculator.install("version", Function::new(VersionSetting));
        calculator.install(
            "update",
            Function::new(UpdateSetting {
                update: Rc::clone(&self.update),
            }),
        );
    }

    pub(crate) fn combination(&self) -> crate::shortcut::Combination {
        *self.combination.borrow()
    }

    pub(crate) fn registered(&self) -> bool {
        self.shortcut.borrow().is_some()
    }

    pub(crate) fn requested(&self) -> bool {
        self.shortcut
            .borrow()
            .as_ref()
            .map(|shortcut| shortcut.requested().is_some())
            .unwrap_or(false)
    }

    pub(crate) fn autostart(&self) -> Option<Rc<crate::autostart::Autostart>> {
        self.autostart.clone()
    }

    pub(crate) fn autostart_enabled(&self) -> bool {
        self.autostart
            .as_ref()
            .map(|autostart| autostart.enabled())
            .unwrap_or(false)
    }

    pub(crate) fn set_update(&self, report: Option<&crate::update::Report>) {
        *self.update.borrow_mut() = report.map(|report| match report {
            crate::update::Report::Updated(version) => {
                format!("updated to {version} — restart to apply")
            }
            crate::update::Report::Current => String::from("up to date"),
            crate::update::Report::Failed(failure) => {
                format!("update check failed: {failure}")
            }
        });
    }
}

struct ShortcutSetting {
    context: eframe::egui::Context,
    combination: Rc<RefCell<crate::shortcut::Combination>>,
    shortcut: Rc<RefCell<Option<crate::shortcut::Shortcut>>>,
}

impl Callable for ShortcutSetting {
    fn name(&self) -> Text {
        Text::new("shortcut")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let key = match values {
            [] => crate::shortcut::Combination::key(&self.combination.borrow()),
            [value] => {
                let key = value
                    .cast::<element::Key>()
                    .ok_or_else(|| Failure::new("shortcut requires a key"))?;
                let combination = crate::shortcut::Combination::from_key(key)?;
                let mut shortcut = self.shortcut.borrow_mut();
                match shortcut.as_mut() {
                    Some(shortcut) => shortcut.replace(combination),
                    None => crate::shortcut::Shortcut::new(&self.context, combination)
                        .map(|registered| *shortcut = Some(registered)),
                }
                .map_err(|error| Failure::new(error.to_string()))?;
                *self.combination.borrow_mut() = combination;
                key.clone()
            }
            _ => return Err(Failure::new("shortcut takes one key")),
        };
        Ok(Value::new(key))
    }
}

struct AutostartSetting {
    autostart: Option<Rc<crate::autostart::Autostart>>,
}

impl Callable for AutostartSetting {
    fn name(&self) -> Text {
        Text::new("autostart")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let autostart = self
            .autostart
            .as_ref()
            .ok_or_else(|| Failure::new("autostart is unavailable"))?;
        match values {
            [] => {}
            [value] => {
                let truth = value
                    .cast::<Truth>()
                    .ok_or_else(|| Failure::new("autostart requires true or false"))?
                    .value();
                if truth {
                    autostart.enable()
                } else {
                    autostart.disable()
                }
                .map_err(|error| Failure::new(error.to_string()))?;
            }
            _ => return Err(Failure::new("autostart takes one truth")),
        }
        Ok(Value::new(Truth::new(autostart.enabled())))
    }
}

struct VersionSetting;

impl Callable for VersionSetting {
    fn name(&self) -> Text {
        Text::new("version")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        if !values.is_empty() {
            return Err(Failure::new("version takes no arguments"));
        }
        Ok(Value::new(Text::new(
            crate::update::Version::current().to_string(),
        )))
    }
}

struct UpdateSetting {
    update: Rc<RefCell<Option<String>>>,
}

impl Callable for UpdateSetting {
    fn name(&self) -> Text {
        Text::new("update")
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        if !values.is_empty() {
            return Err(Failure::new("update takes no arguments"));
        }
        let state = self
            .update
            .borrow()
            .clone()
            .unwrap_or_else(|| String::from("checking"));
        Ok(Value::new(Text::new(state)))
    }
}
