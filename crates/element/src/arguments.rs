pub struct Arguments<'a> {
    values: &'a [crate::Value],
}

impl<'a> Arguments<'a> {
    pub fn new(values: &'a [crate::Value]) -> Self {
        Self { values }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&'a crate::Value> {
        self.values.get(index)
    }

    pub fn number(&self, index: usize) -> Result<crate::Number, crate::Failure> {
        self.get(index)
            .and_then(|value| value.cast::<crate::Number>())
            .copied()
            .ok_or_else(|| self.mismatch(index, "a number"))
    }

    pub fn truth(&self, index: usize) -> Result<bool, crate::Failure> {
        self.get(index)
            .and_then(|value| value.cast::<crate::Truth>())
            .map(|truth| truth.value())
            .ok_or_else(|| self.mismatch(index, "a true or false"))
    }

    pub fn key(&self, index: usize) -> Result<&'a crate::Key, crate::Failure> {
        self.get(index)
            .and_then(|value| value.cast::<crate::Key>())
            .ok_or_else(|| self.mismatch(index, "a key"))
    }

    pub fn numbers(&self) -> Result<Vec<crate::Number>, crate::Failure> {
        self.values
            .iter()
            .map(|value| {
                value
                    .cast::<crate::Number>()
                    .copied()
                    .ok_or_else(|| crate::Failure::Type {
                        expected: crate::Text::new("a number"),
                        actual: value.describe(),
                    })
            })
            .collect()
    }

    fn mismatch(&self, index: usize, expected: &str) -> crate::Failure {
        crate::Failure::Type {
            expected: crate::Text::new(expected),
            actual: self
                .get(index)
                .map(|value| value.describe())
                .unwrap_or_else(|| crate::Text::new("a value")),
        }
    }
}
