
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use element::{Arity, Arguments, Callable, Failure, Function, Text, Truth, Value};
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};

use crate::autostart::Autostart;

#[derive(Clone)]
pub struct Settings {
    manager: Option<Rc<GlobalHotKeyManager>>,
    current: Rc<RefCell<HotKey>>,
    registered: Rc<Cell<bool>>,
    autostart: Option<Rc<Autostart>>,
    autostart_listener: Rc<RefCell<Option<Box<dyn Fn(bool)>>>>,
    update: crate::update::Update,
}

impl Settings {
    pub fn new(wake: impl Fn() + Send + 'static) -> Self {
        let default = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyC);
        let stored = SettingsFile::shortcut()
            .and_then(|text| Hotkey::parse(&text))
            .map(|hotkey| hotkey.0)
            .unwrap_or(default);
        let manager = GlobalHotKeyManager::new().ok().map(Rc::new);
        let registered = manager
            .as_ref()
            .is_some_and(|manager| manager.register(stored).is_ok());
        Self {
            manager,
            current: Rc::new(RefCell::new(stored)),
            registered: Rc::new(Cell::new(registered)),
            autostart: Autostart::new().ok().map(Rc::new),
            autostart_listener: Rc::new(RefCell::new(None)),
            update: crate::update::Update::new(wake),
        }
    }

    pub fn install(&self, calculator: &mut evaluator::Calculator) {
        calculator.install(
            "shortcut",
            Function::new(ShortcutSetting {
                settings: self.clone(),
            }),
        );
        calculator.install(
            "autostart",
            Function::new(AutostartSetting {
                settings: self.clone(),
            }),
        );
        calculator.install("version", Function::new(VersionSetting));
        calculator.install(
            "update",
            Function::new(UpdateSetting {
                update: self.update.clone(),
            }),
        );
    }

    pub fn on_autostart(&self, listener: impl Fn(bool) + 'static) {
        *self.autostart_listener.borrow_mut() = Some(Box::new(listener));
    }

    pub fn text(&self) -> Text {
        Hotkey::new(*self.current.borrow()).text()
    }

    pub fn registered(&self) -> bool {
        self.registered.get()
    }

    pub fn autostart_enabled(&self) -> bool {
        self.autostart
            .as_ref()
            .is_some_and(|autostart| autostart.enabled())
    }

    pub fn set_autostart(&self, enabled: bool) -> Result<bool, Failure> {
        self.autostart(Some(enabled))
    }

    pub fn update_state(&self) -> Text {
        self.update.text()
    }

    fn replace(&self, hotkey: HotKey) -> Result<(), Failure> {
        let Some(manager) = &self.manager else {
            return Err(Failure::new("global shortcuts are unavailable"));
        };
        let previous = *self.current.borrow();
        let was_registered = self.registered.get();
        let _ = manager.unregister(previous);
        match manager.register(hotkey) {
            Ok(()) => {
                *self.current.borrow_mut() = hotkey;
                self.registered.set(true);
                SettingsFile::store(&Hotkey::new(hotkey).text());
                Ok(())
            }
            Err(error) => {
                let _ = manager.register(previous);
                self.registered.set(was_registered);
                Err(Failure::new(error.to_string()))
            }
        }
    }

    fn autostart(&self, value: Option<bool>) -> Result<bool, Failure> {
        let autostart = self
            .autostart
            .as_ref()
            .ok_or_else(|| Failure::new("autostart is unavailable"))?;
        if let Some(enabled) = value {
            if enabled {
                autostart.enable()
            } else {
                autostart.disable()
            }
            .map_err(|error| Failure::new(error.to_string()))?;
        }
        let enabled = autostart.enabled();
        if value.is_some()
            && let Some(listener) = self.autostart_listener.borrow().as_ref()
        {
            listener(enabled);
        }
        Ok(enabled)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new(|| {})
    }
}

struct ShortcutSetting {
    settings: Settings,
}

impl Callable for ShortcutSetting {
    fn name(&self) -> Text {
        Text::new("shortcut")
    }

    fn arity(&self) -> Arity {
        Arity::between(0, 1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let key = if values.is_empty() {
            Hotkey::new(*self.settings.current.borrow()).key()
        } else {
            let key = arguments.key(0)?;
            let hotkey = Hotkey::parse(&Text::new(key.to_string()))
                .ok_or_else(|| Failure::new("not a key combination"))?;
            self.settings.replace(hotkey.0)?;
            key.clone()
        };
        Ok(Value::new(key))
    }
}

struct AutostartSetting {
    settings: Settings,
}

impl Callable for AutostartSetting {
    fn name(&self) -> Text {
        Text::new("autostart")
    }

    fn arity(&self) -> Arity {
        Arity::between(0, 1)
    }

    fn call(&self, values: &[Value]) -> Result<Value, Failure> {
        let arguments = Arguments::new(values);
        let requested = if values.is_empty() {
            None
        } else {
            Some(arguments.truth(0)?)
        };
        Ok(Value::new(Truth::new(
            self.settings.autostart(requested)?,
        )))
    }
}

struct VersionSetting;

impl Callable for VersionSetting {
    fn name(&self) -> Text {
        Text::new("version")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(0)
    }

    fn call(&self, _values: &[Value]) -> Result<Value, Failure> {
        Ok(Value::new(Text::new(env!("CARGO_PKG_VERSION"))))
    }
}

struct UpdateSetting {
    update: crate::update::Update,
}

impl Callable for UpdateSetting {
    fn name(&self) -> Text {
        Text::new("update")
    }

    fn arity(&self) -> Arity {
        Arity::exactly(0)
    }

    fn call(&self, _values: &[Value]) -> Result<Value, Failure> {
        Ok(Value::new(self.update.text()))
    }
}

struct Hotkey(HotKey);

impl Hotkey {
    fn new(hotkey: HotKey) -> Self {
        Self(hotkey)
    }

    fn key(&self) -> element::Key {
        let mut modifiers = element::Modifiers::NONE;
        for (source, target) in [
            (Modifiers::CONTROL, element::Modifiers::CONTROL),
            (Modifiers::ALT, element::Modifiers::ALT),
            (Modifiers::SHIFT, element::Modifiers::SHIFT),
            (Modifiers::META, element::Modifiers::SUPER),
        ] {
            if self.0.mods.contains(source) {
                modifiers = modifiers.union(target);
            }
        }
        element::Key::new(modifiers, Self::name(self.0.key))
    }

    fn text(&self) -> Text {
        let mut text = String::new();
        for (modifier, name) in [
            (Modifiers::CONTROL, "Ctrl"),
            (Modifiers::ALT, "Alt"),
            (Modifiers::SHIFT, "Shift"),
            (Modifiers::META, "Super"),
        ] {
            if self.0.mods.contains(modifier) {
                if !text.is_empty() {
                    text.push('+');
                }
                text.push_str(name);
            }
        }
        if !text.is_empty() {
            text.push('+');
        }
        text.push_str(Self::name(self.0.key).as_ref());
        Text::new(text)
    }

    fn parse(text: &Text) -> Option<Self> {
        let normalized = text
            .as_ref()
            .split('+')
            .map(|part| {
                let token = part.trim();
                if token.eq_ignore_ascii_case("ctrl") || token.eq_ignore_ascii_case("control") {
                    String::from("Ctrl")
                } else if token.eq_ignore_ascii_case("alt") {
                    String::from("Alt")
                } else if token.eq_ignore_ascii_case("shift") {
                    String::from("Shift")
                } else if token.eq_ignore_ascii_case("super")
                    || token.eq_ignore_ascii_case("meta")
                    || token.eq_ignore_ascii_case("cmd")
                    || token.eq_ignore_ascii_case("command")
                {
                    String::from("Super")
                } else if token.len() == 1 && token.chars().all(|c| c.is_ascii_alphabetic()) {
                    format!("Key{}", token.to_ascii_uppercase())
                } else if token.len() == 1 && token.chars().all(|c| c.is_ascii_digit()) {
                    format!("Digit{token}")
                } else if token
                    .get(..3)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("key"))
                {
                    format!("Key{}", &token[3..].to_ascii_uppercase())
                } else if token
                    .get(..5)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("digit"))
                {
                    format!("Digit{}", &token[5..])
                } else {
                    token.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("+");
        let hotkey: HotKey = normalized.parse().ok()?;
        (!hotkey.mods.is_empty()).then(|| Self::new(hotkey))
    }

    fn name(code: Code) -> Text {
        let name = format!("{code:?}");
        Text::new(
            name.strip_prefix("Key")
                .or_else(|| name.strip_prefix("Digit"))
                .unwrap_or(&name),
        )
    }
}

struct SettingsFile;

impl SettingsFile {
    fn path() -> Option<std::path::PathBuf> {
        let base = directories::BaseDirs::new()?;
        Some(base.data_dir().join("bullsheet").join("settings"))
    }

    fn shortcut() -> Option<Text> {
        let contents = std::fs::read_to_string(Self::path()?).ok()?;
        contents.lines().find_map(|line| {
            line.strip_prefix("shortcut")
                .and_then(|rest| rest.strip_prefix('='))
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(Text::new)
        })
    }

    fn store(shortcut: &Text) {
        let Some(path) = Self::path() else {
            return;
        };
        if let Some(directory) = path.parent() {
            let _ = std::fs::create_dir_all(directory);
        }
        let _ = std::fs::write(path, format!("shortcut = {shortcut}\n"));
    }
}
