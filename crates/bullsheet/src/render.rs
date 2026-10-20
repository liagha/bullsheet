#[derive(Clone, Debug)]
pub struct Presentation(String);

impl Presentation {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    pub fn number(value: f64) -> Self {
        Self(Self::formatted(value, Some(',')))
    }

    pub fn plain(value: f64) -> Self {
        Self(Self::formatted(value, None))
    }

    fn formatted(value: f64, separator: Option<char>) -> String {
        if value.is_nan() {
            return String::from("NaN");
        }
        if value.is_infinite() {
            return if value.is_sign_positive() {
                String::from("∞")
            } else {
                String::from("-∞")
            };
        }
        if value == 0.0 {
            return String::from("0");
        }
        let magnitude = value.abs();
        if magnitude >= 1.0e15 || magnitude < 1.0e-9 {
            return format!("{:e}", value);
        }
        let rounded = (value * 1.0e12).round() / 1.0e12;
        if rounded == 0.0 {
            return String::from("0");
        }
        let mut text = format!("{:.12}", rounded);
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        match separator {
            Some(separator) => match text.split_once('.') {
                Some((integer, fraction)) => {
                    format!("{}{}{}", Self::grouped(integer), separator, fraction)
                }
                None => Self::grouped(&text),
            },
            None => text,
        }
    }

    fn grouped(integer: &str) -> String {
        let (sign, digits) = match integer.strip_prefix('-') {
            Some(digits) => ("-", digits),
            None => ("", integer),
        };
        let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
        for (position, character) in digits.chars().enumerate() {
            if position > 0 && (digits.len() - position) % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(character);
        }
        let mut result = String::with_capacity(sign.len() + grouped.len());
        result.push_str(sign);
        result.push_str(&grouped);
        result
    }
}

impl std::fmt::Display for Presentation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for Presentation {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for Presentation {
    fn from(text: String) -> Self {
        Self(text)
    }
}

pub struct Shown {
    pub text: Presentation,
    pub color: eframe::egui::Color32,
    pub plain: Option<Presentation>,
}

pub trait Render {
    fn render(&self) -> Option<Shown>;
}

impl Render for element::Number {
    fn render(&self) -> Option<Shown> {
        Some(Shown {
            text: Presentation::number(self.value()),
            color: eframe::egui::Color32::from_rgb(126, 231, 135),
            plain: Some(Presentation::plain(self.value())),
        })
    }
}

impl Render for element::Text {
    fn render(&self) -> Option<Shown> {
        let text = Presentation::new(self.as_ref());
        Some(Shown {
            text: text.clone(),
            color: eframe::egui::Color32::from_rgb(224, 227, 233),
            plain: Some(text),
        })
    }
}

impl Render for element::Truth {
    fn render(&self) -> Option<Shown> {
        let text = Presentation::new(self.value().to_string());
        Some(Shown {
            text: text.clone(),
            color: eframe::egui::Color32::from_rgb(126, 231, 135),
            plain: Some(text),
        })
    }
}

impl Render for element::Key {
    fn render(&self) -> Option<Shown> {
        let text = Presentation::new(self.to_string());
        Some(Shown {
            text: text.clone(),
            color: eframe::egui::Color32::from_rgb(150, 190, 255),
            plain: Some(text),
        })
    }
}

impl Render for element::Function {
    fn render(&self) -> Option<Shown> {
        let text = Presentation::new(self.name().as_ref());
        Some(Shown {
            text: text.clone(),
            color: eframe::egui::Color32::from_rgb(198, 160, 246),
            plain: Some(text),
        })
    }
}

impl Render for element::Blank {
    fn render(&self) -> Option<Shown> {
        None
    }
}

impl Render for element::Parameter {
    fn render(&self) -> Option<Shown> {
        let text = Presentation::new(self.name().as_ref());
        Some(Shown {
            text: text.clone(),
            color: eframe::egui::Color32::from_rgb(170, 176, 188),
            plain: Some(text),
        })
    }
}

impl Render for element::Failure {
    fn render(&self) -> Option<Shown> {
        Some(Shown {
            text: Presentation::new(self.to_string()),
            color: eframe::egui::Color32::from_rgb(255, 123, 114),
            plain: None,
        })
    }
}

pub struct Renderers {
    entries: std::collections::HashMap<std::any::TypeId, fn(&dyn std::any::Any) -> Option<Shown>>,
}

impl Renderers {
    pub fn new() -> Self {
        let mut renderers = Self {
            entries: std::collections::HashMap::new(),
        };
        renderers.learn::<element::Number>();
        renderers.learn::<element::Text>();
        renderers.learn::<element::Truth>();
        renderers.learn::<element::Key>();
        renderers.learn::<element::Function>();
        renderers.learn::<element::Blank>();
        renderers.learn::<element::Failure>();
        renderers.learn::<element::Parameter>();
        renderers
    }

    fn learn<T: Render + 'static>(&mut self) {
        self.entries.insert(std::any::TypeId::of::<T>(), |value| {
            value.downcast_ref::<T>().and_then(Render::render)
        });
    }

    pub fn render(&self, value: &element::Value) -> Option<Shown> {
        let any = value.any();
        self.entries
            .get(&any.type_id())
            .and_then(|render| render(any))
    }
}
