use unicode_script::{Script, UnicodeScript};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Writing {
    Latin,
    Persian,
}

impl Writing {
    pub fn of(character: char) -> Option<Self> {
        match character.script() {
            Script::Arabic => Some(Self::Persian),
            Script::Latin => Some(Self::Latin),
            _ => None,
        }
    }

    pub fn of_text(text: &str) -> Self {
        let characters: Vec<char> = text.chars().collect();
        if Self::each(&characters)
            .iter()
            .any(|writing| *writing == Self::Persian)
        {
            Self::Persian
        } else {
            Self::Latin
        }
    }

    pub fn each(characters: &[char]) -> Vec<Self> {
        let strong: Vec<Option<Self>> = characters
            .iter()
            .map(|character| Self::of(*character))
            .collect();
        let mut writings = Vec::with_capacity(characters.len());
        let mut previous: Option<Self> = None;
        for (index, writing) in strong.iter().enumerate() {
            let writing = match writing {
                Some(writing) => *writing,
                None => previous
                    .or_else(|| {
                        strong[index + 1..]
                            .iter()
                            .filter_map(|writing| *writing)
                            .next()
                    })
                    .unwrap_or(Self::Latin),
            };
            if strong[index].is_some() {
                previous = Some(writing);
            }
            writings.push(writing);
        }
        writings
    }
}
