/// Pointcut: resolves `"beanName::methodName"` pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pointcut {
    pub bean_name: String,
    pub method_name: String,
}

impl Pointcut {
    /// Parse a pointcut expression of the form `"beanName::methodName"`.
    pub fn parse(expr: &str) -> Result<Self, String> {
        let mut parts = expr.splitn(2, "::");
        let bean_name = parts.next().unwrap_or("").trim();
        let method_name = parts.next().unwrap_or("").trim();
        if bean_name.is_empty() || method_name.is_empty() {
            return Err(format!(
                "invalid pointcut expression '{}': expected 'beanName::methodName'",
                expr
            ));
        }
        Ok(Pointcut {
            bean_name: bean_name.to_string(),
            method_name: method_name.to_string(),
        })
    }

    /// Returns `true` when this pointcut matches the given bean + method.
    pub fn matches(&self, bean_name: &str, method_name: &str) -> bool {
        self.bean_name == bean_name && self.method_name == method_name
    }
}

#[cfg(test)]
mod tests {
    use super::Pointcut;

    #[test]
    fn parse_valid_pointcut() {
        let p = Pointcut::parse("userService::save").expect("valid pointcut should parse");
        assert_eq!(p.bean_name, "userService");
        assert_eq!(p.method_name, "save");
        assert!(p.matches("userService", "save"));
    }

    #[test]
    fn parse_rejects_missing_separator() {
        assert!(Pointcut::parse("userService_save").is_err());
    }

    #[test]
    fn parse_rejects_empty_segments() {
        assert!(Pointcut::parse("::save").is_err());
        assert!(Pointcut::parse("userService::").is_err());
        assert!(Pointcut::parse("::").is_err());
    }
}
