#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutowireError {
    pub bean_name: String,
    pub field_name: String,
}

impl AutowireError {
    pub fn new(bean_name: impl Into<String>, field_name: impl Into<String>) -> Self {
        Self {
            bean_name: bean_name.into(),
            field_name: field_name.into(),
        }
    }
}

impl std::fmt::Display for AutowireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to autowire field '{}' for bean '{}'",
            self.field_name, self.bean_name
        )
    }
}

impl std::error::Error for AutowireError {}
