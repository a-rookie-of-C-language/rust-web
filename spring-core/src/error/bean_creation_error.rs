#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeanCreationError {
    pub bean_name: String,
    pub reason: String,
}

impl BeanCreationError {
    pub fn new(bean_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            bean_name: bean_name.into(),
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for BeanCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to create bean '{}': {}",
            self.bean_name, self.reason
        )
    }
}

impl std::error::Error for BeanCreationError {}
