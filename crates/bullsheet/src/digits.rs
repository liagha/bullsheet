use element::Text;

pub struct Digits;

impl Digits {
    pub fn to_latin(text: &Text) -> Text {
        Text::new(
            text.as_ref()
                .chars()
                .map(|character| match character {
                    '۰'..='۹' => {
                        char::from_digit(character as u32 - '۰' as u32, 10).unwrap_or(character)
                    }
                    '٠'..='٩' => {
                        char::from_digit(character as u32 - '٠' as u32, 10).unwrap_or(character)
                    }
                    '٬' | '،' => ',',
                    '٫' => '.',
                    _ => character,
                })
                .collect::<String>(),
        )
    }
}
