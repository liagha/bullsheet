#[derive(Clone, Copy, Debug)]
pub(crate) struct Combination(global_hotkey::hotkey::HotKey);

impl Default for Combination {
    fn default() -> Self {
        Self(global_hotkey::hotkey::HotKey::new(
            Some(
                global_hotkey::hotkey::Modifiers::CONTROL
                    | global_hotkey::hotkey::Modifiers::ALT,
            ),
            global_hotkey::hotkey::Code::KeyC,
        ))
    }
}

impl std::fmt::Display for Combination {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use global_hotkey::hotkey::Modifiers;
        let modifiers = self.0.mods;
        let mut text = String::new();
        if modifiers.contains(Modifiers::CONTROL) {
            text.push_str("Ctrl+");
        }
        if modifiers.contains(Modifiers::ALT) {
            text.push_str("Alt+");
        }
        if modifiers.contains(Modifiers::SHIFT) {
            text.push_str("Shift+");
        }
        if modifiers.contains(Modifiers::META) {
            text.push_str("Super+");
        }
        text.push_str(&Self::key_name(self.0.key));
        formatter.write_str(&text)
    }
}

impl std::str::FromStr for Combination {
    type Err = global_hotkey::hotkey::HotKeyParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        use global_hotkey::hotkey::HotKey;
        let normalized = Self::normalize(text);
        let hotkey =
            <HotKey as std::str::FromStr>::from_str(&normalized)?;
        if hotkey.mods.is_empty() {
            return Err(
                global_hotkey::hotkey::HotKeyParseError::InvalidFormat(
                    text.to_string(),
                ),
            );
        }
        Ok(Self(hotkey))
    }
}

impl Combination {
    fn key_name(key: global_hotkey::hotkey::Code) -> String {
        let name = format!("{key:?}");
        name.strip_prefix("Key")
            .or_else(|| name.strip_prefix("Digit"))
            .unwrap_or(&name)
            .to_string()
    }

    fn normalize(text: &str) -> String {
        text.split('+')
            .map(|part| {
                let token = part.trim();
                if token.eq_ignore_ascii_case("ctrl")
                    || token.eq_ignore_ascii_case("control")
                {
                    "Ctrl".to_string()
                } else if token.eq_ignore_ascii_case("alt") {
                    "Alt".to_string()
                } else if token.eq_ignore_ascii_case("shift") {
                    "Shift".to_string()
                } else if token.eq_ignore_ascii_case("super")
                    || token.eq_ignore_ascii_case("meta")
                    || token.eq_ignore_ascii_case("cmd")
                    || token.eq_ignore_ascii_case("command")
                {
                    "Super".to_string()
                } else if token.len() == 1
                    && token
                        .chars()
                        .all(|character| character.is_ascii_alphabetic())
                {
                    format!("Key{}", token.to_ascii_uppercase())
                } else if token.len() == 1
                    && token
                        .chars()
                        .all(|character| character.is_ascii_digit())
                {
                    format!("Digit{token}")
                } else if token
                    .get(..3)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("key"))
                {
                    format!("Key{}", token[3..].to_ascii_uppercase())
                } else if token
                    .get(..5)
                    .is_some_and(|prefix| {
                        prefix.eq_ignore_ascii_case("digit")
                    })
                {
                    format!("Digit{}", &token[5..])
                } else {
                    token.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("+")
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Toggle;

#[derive(Debug)]
pub(crate) struct ShortcutFailure(String);

impl ShortcutFailure {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl std::fmt::Display for ShortcutFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ShortcutFailure {}

pub(crate) struct Shortcut {
    manager: global_hotkey::GlobalHotKeyManager,
    combination: Combination,
    identifier: std::sync::Arc<std::sync::atomic::AtomicU32>,
    receiver: std::sync::mpsc::Receiver<Toggle>,
}

impl Shortcut {
    pub(crate) fn new(
        context: &eframe::egui::Context,
        combination: Combination,
    ) -> Result<Self, ShortcutFailure> {
        let manager = global_hotkey::GlobalHotKeyManager::new()
            .map_err(|error| ShortcutFailure::new(error.to_string()))?;
        manager
            .register(combination.0)
            .map_err(|error| ShortcutFailure::new(error.to_string()))?;
        let (sender, receiver) = std::sync::mpsc::channel();
        let context = context.clone();
        let identifier = std::sync::Arc::new(
            std::sync::atomic::AtomicU32::new(combination.0.id()),
        );
        let thread_identifier = identifier.clone();
        std::thread::spawn(move || {
            let events = global_hotkey::GlobalHotKeyEvent::receiver();
            while let Ok(event) = events.recv() {
                if event.id
                    == thread_identifier
                        .load(std::sync::atomic::Ordering::Relaxed)
                    && event.state == global_hotkey::HotKeyState::Pressed
                {
                    if sender.send(Toggle).is_err() {
                        break;
                    }
                    context.request_repaint();
                }
            }
        });
        Ok(Self {
            manager,
            combination,
            identifier,
            receiver,
        })
    }

    pub(crate) fn replace(
        &mut self,
        combination: Combination,
    ) -> Result<(), ShortcutFailure> {
        let previous = self.combination;
        let _ = self.manager.unregister(previous.0);
        match self.manager.register(combination.0) {
            Ok(()) => {
                self.combination = combination;
                self.identifier.store(
                    combination.0.id(),
                    std::sync::atomic::Ordering::Relaxed,
                );
                Ok(())
            }
            Err(error) => {
                let _ = self.manager.register(previous.0);
                Err(ShortcutFailure::new(error.to_string()))
            }
        }
    }

    pub(crate) fn requested(&self) -> Option<Toggle> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for Shortcut {
    fn drop(&mut self) {
        let _ = self.manager.unregister(self.combination.0);
    }
}
