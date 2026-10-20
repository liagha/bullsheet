#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const NONE: Self = Self(0);
    pub const CONTROL: Self = Self(1);
    pub const ALT: Self = Self(2);
    pub const SHIFT: Self = Self(4);
    pub const SUPER: Self = Self(8);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Clone, Debug)]
pub struct Key {
    modifiers: Modifiers,
    code: Option<crate::Text>,
}

impl Key {
    pub fn new(modifiers: Modifiers, code: crate::Text) -> Self {
        Self {
            modifiers,
            code: Some(code),
        }
    }

    pub fn modifier(modifiers: Modifiers) -> Self {
        Self {
            modifiers,
            code: None,
        }
    }

    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    pub fn code(&self) -> Option<&str> {
        self.code.as_ref().map(|code| code.as_ref())
    }

    pub fn combine(&self, other: &Self) -> Result<Self, crate::Failure> {
        let code = match (&self.code, &other.code) {
            (Some(_), Some(_)) => {
                return Err(crate::Failure::new(
                    "a key combination holds one key",
                ));
            }
            (Some(code), None) | (None, Some(code)) => Some(code.clone()),
            (None, None) => None,
        };
        Ok(Self {
            modifiers: self.modifiers.union(other.modifiers),
            code,
        })
    }
}

impl crate::Element for Key {}

impl std::fmt::Display for Key {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut text = String::new();
        for (modifier, name) in [
            (Modifiers::CONTROL, "Ctrl"),
            (Modifiers::ALT, "Alt"),
            (Modifiers::SHIFT, "Shift"),
            (Modifiers::SUPER, "Super"),
        ] {
            if self.modifiers.contains(modifier) {
                if !text.is_empty() {
                    text.push('+');
                }
                text.push_str(name);
            }
        }
        if let Some(code) = &self.code {
            if !text.is_empty() {
                text.push('+');
            }
            text.push_str(code.as_ref());
        }
        formatter.write_str(&text)
    }
}
