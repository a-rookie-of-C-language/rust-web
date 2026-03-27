#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyAccessError {
    pub key: String,
    pub detail: String,
}

impl PropertyAccessError {
    pub fn new(key: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for PropertyAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to resolve property '{}': {}",
            self.key, self.detail
        )
    }
}

impl std::error::Error for PropertyAccessError {}
