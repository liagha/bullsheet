use element::Failure;

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

impl TryFrom<&str> for Identifier {
    type Error = Failure;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let trimmed = text.trim();
        let mut characters = trimmed.chars();
        match characters.next() {
            Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
            _ => return Err(Failure::new("invalid name")),
        }
        if characters.any(|character| !(character.is_ascii_alphanumeric() || character == '_')) {
            return Err(Failure::new("invalid name"));
        }
        Ok(Self(trimmed.to_ascii_lowercase()))
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

    pub fn statement(&self) -> Result<Statement, Failure> {
        let code = self.without_comment().trim();
        if code.is_empty() {
            return Ok(Statement::Empty);
        }
        let offset = self.text.chars().count() - self.text.trim_start().chars().count();
        if let Some((left, right)) = Self::assignment(code) {
            let expression = Expression::new(right, offset + left.chars().count() + 1);
            if expression.is_empty() {
                return Err(Failure::new("missing expression"));
            }
            if let Some((name, span, parameters)) = Self::definition(left, offset)? {
                return Ok(Statement::Definition {
                    name,
                    span,
                    parameters,
                    expression,
                });
            }
            let name = Identifier::try_from(left)?;
            let start = offset + left.chars().count() - left.trim_start().chars().count();
            let span = start..start + left.trim().chars().count();
            return Ok(Statement::Assignment {
                name,
                span,
                expression,
            });
        }
        let expression = Expression::new(code, offset);
        if expression.is_empty() {
            return Ok(Statement::Empty);
        }
        Ok(Statement::Expression(expression))
    }

    fn assignment(code: &str) -> Option<(&str, &str)> {
        let characters: Vec<char> = code.chars().collect();
        let mut depth = 0usize;
        for (index, character) in characters.iter().enumerate() {
            match character {
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                '=' if depth == 0 => {
                    let previous = index.checked_sub(1).map(|earlier| characters[earlier]);
                    let next = characters.get(index + 1);
                    if matches!(previous, Some('=' | '<' | '>' | '!')) || next == Some(&'=') {
                        continue;
                    }
                    let byte = code.char_indices().nth(index)?.0;
                    return Some((&code[..byte], &code[byte + 1..]));
                }
                _ => {}
            }
        }
        None
    }

    fn definition(
        left: &str,
        offset: usize,
    ) -> Result<Option<(Identifier, Span, Vec<(Identifier, Span)>)>, Failure> {
        let start = offset + left.chars().count() - left.trim_start().chars().count();
        let left = left.trim();
        if !left.ends_with(')') {
            return Ok(None);
        }
        let open = match left.find('(') {
            Some(open) => open,
            None => return Ok(None),
        };
        let name = Identifier::try_from(left[..open].trim())?;
        let span = start..start + left[..open].trim().chars().count();
        let mut parameters = Vec::new();
        let mut cursor = start + left[..open].chars().count() + 1;
        for piece in left[open + 1..left.len() - 1].split(',') {
            let parameter = piece.trim();
            if !parameter.is_empty() {
                let begin = cursor + piece.chars().count() - piece.trim_start().chars().count();
                parameters.push((
                    Identifier::try_from(parameter)?,
                    begin..begin + parameter.chars().count(),
                ));
            }
            cursor += piece.chars().count() + 1;
        }
        Ok(Some((name, span, parameters)))
    }

    fn without_comment(&self) -> &'a str {
        self.text
            .split_once('#')
            .map_or(self.text, |(code, _)| code)
    }
}

#[derive(Debug)]
pub struct Expression {
    text: String,
    offset: usize,
    sources: Vec<usize>,
}

impl Expression {
    pub fn new(text: &str, offset: usize) -> Self {
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
            offset,
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
        (self.offset + start)..(self.offset + end)
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

#[derive(Debug)]
pub enum Statement {
    Empty,
    Expression(Expression),
    Assignment {
        name: Identifier,
        span: Span,
        expression: Expression,
    },
    Definition {
        name: Identifier,
        span: Span,
        parameters: Vec<(Identifier, Span)>,
        expression: Expression,
    },
}
