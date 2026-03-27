#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoSuchBeanError {
    pub bean_name: String,
}

impl NoSuchBeanError {
    pub fn new(bean_name: impl Into<String>) -> Self {
        Self {
            bean_name: bean_name.into(),
        }
    }
}

impl std::fmt::Display for NoSuchBeanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bean '{}' does not exist", self.bean_name)
    }
}

impl std::error::Error for NoSuchBeanError {}
