pub type Span = std::ops::Range<usize>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Identifier(String);

impl Identifier {
    pub fn new(text: &str) -> Self {
        Self(text.to_ascii_lowercase())
    }

    pub fn is_answer(&self) -> bool {
        self.0 == "ans"
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug)]
pub struct Source(String);

impl Source {
    pub fn new(text: String) -> Self {
        Self(text)
    }

    pub fn line_count(&self) -> usize {
        self.0.split('\n').count()
    }

    pub fn lines(&self) -> impl Iterator<Item = Line<'_>> {
        self.0.split('\n').map(Line::new)
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for Source {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsMut<String> for Source {
    fn as_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl From<String> for Source {
    fn from(text: String) -> Self {
        Self(text)
    }
}

impl From<&str> for Source {
    fn from(text: &str) -> Self {
        Self(text.to_string())
    }
}

pub struct Line<'a> {
    text: &'a str,
}

impl<'a> Line<'a> {
    fn new(text: &'a str) -> Self {
        Self { text }
    }

    pub fn expression(&self) -> Expression {
        Expression::new(self.text)
    }
}

#[derive(Debug)]
pub struct Expression {
    text: String,
    sources: Vec<usize>,
}

impl Expression {
    pub fn new(text: &str) -> Self {
        let text = text.split_once('#').map_or(text, |(code, _)| code);
        let mut normalized = String::with_capacity(text.len());
        let mut sources = Vec::with_capacity(text.len());
        for (index, character) in text.chars().enumerate() {
            match character {
                '\u{06F0}'..='\u{06F9}' => {
                    let digit = char::from_u32('0' as u32 + (character as u32 - '\u{06F0}' as u32))
                        .unwrap_or('0');
                    normalized.push(digit);
                    sources.push(index);
                }
                '\u{0660}'..='\u{0669}' => {
                    let digit = char::from_u32('0' as u32 + (character as u32 - '\u{0660}' as u32))
                        .unwrap_or('0');
                    normalized.push(digit);
                    sources.push(index);
                }
                '×' | '·' => {
                    normalized.push('*');
                    sources.push(index);
                }
                '÷' => {
                    normalized.push('/');
                    sources.push(index);
                }
                '−' | '–' | '—' => {
                    normalized.push('-');
                    sources.push(index);
                }
                '٬' | '،' => {
                    normalized.push(',');
                    sources.push(index);
                }
                '٫' => {
                    normalized.push('.');
                    sources.push(index);
                }
                '٪' => {
                    normalized.push('%');
                    sources.push(index);
                }
                'π' => Self::expanded(&mut normalized, &mut sources, "pi", index),
                'τ' => Self::expanded(&mut normalized, &mut sources, "tau", index),
                '∞' => Self::expanded(&mut normalized, &mut sources, "inf", index),
                '\u{200E}' | '\u{200F}' => {}
                _ => {
                    normalized.push(character);
                    sources.push(index);
                }
            }
        }
        let trimmed = normalized.trim();
        let leading = normalized.chars().count() - normalized.trim_start().chars().count();
        let trailing = normalized.chars().count() - normalized.trim_end().chars().count();
        sources.drain(..leading);
        sources.truncate(sources.len().saturating_sub(trailing));
        Self {
            text: trimmed.to_string(),
            sources,
        }
    }

    fn expanded(normalized: &mut String, sources: &mut Vec<usize>, text: &str, index: usize) {
        for _ in text.chars() {
            sources.push(index);
        }
        normalized.push_str(text);
    }

    pub fn span(&self, span: Span) -> Span {
        let start = self.sources.get(span.start).copied().unwrap_or(0);
        let end = span
            .end
            .checked_sub(1)
            .and_then(|last| self.sources.get(last))
            .map_or(start, |last| last + 1);
        start..end
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.text)
    }
}

impl AsRef<str> for Expression {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

