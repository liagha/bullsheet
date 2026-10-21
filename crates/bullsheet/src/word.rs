use std::ops::Range;

use element::Text;

pub struct Word {
    range: Range<usize>,
    text: Text,
}

impl Word {
    pub fn ending_at(text: &Text, cursor: usize) -> Option<Self> {
        let source = text.as_ref();
        if cursor > source.len() || !source.is_char_boundary(cursor) {
            return None;
        }
        let bytes = source.as_bytes();
        let mut start = cursor;
        while start > 0 {
            let byte = bytes[start - 1];
            if byte.is_ascii_alphanumeric() || byte == b'_' {
                start -= 1;
            } else {
                break;
            }
        }
        if start == cursor {
            return None;
        }
        Some(Self {
            range: start..cursor,
            text: Text::new(&source[start..cursor]),
        })
    }

    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    pub fn text(&self) -> &Text {
        &self.text
    }
}
