use std::collections::HashMap;

/// Central environment abstraction — holds all resolved key→value properties.
/// Mirrors Spring's `Environment` interface.
#[derive(Debug, Default, Clone)]
pub struct Environment {
    properties: HashMap<String, String>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    /// Get a property by key, returns `None` if not found.
    pub fn get_property(&self, key: &str) -> Option<&str> {
        self.properties.get(key).map(|s| s.as_str())
    }

    /// Get a property by key, falling back to `default` if not found.
    pub fn get_property_or_default<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.properties
            .get(key)
            .map(|s| s.as_str())
            .unwrap_or(default)
    }

    /// Set a property.
    pub fn set_property(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.properties.insert(key.into(), value.into());
    }

    /// Merge all entries from a property source into this environment.
    pub fn merge_from(&mut self, source: &dyn super::property_source::PropertySource) {
        for (k, v) in source.get_properties() {
            self.properties
                .entry(k.to_string())
                .or_insert_with(|| v.to_string());
        }
    }

    pub fn merge_from_override(&mut self, source: &dyn super::property_source::PropertySource) {
        for (k, v) in source.get_properties() {
            self.properties.insert(k.to_string(), v.to_string());
        }
    }

    /// Resolve a `${key:default}` or `${key}` placeholder.
    /// Returns the value string if resolved, or `None` if key absent and no default.
    pub fn resolve_placeholder<'a>(&'a self, placeholder: &'a str) -> Option<String> {
        // Strip ${ and }
        let inner = placeholder
            .strip_prefix("${")
            .and_then(|s| s.strip_suffix('}'))?;
        if let Some(colon_pos) = inner.find(':') {
            let key = &inner[..colon_pos];
            let default = &inner[colon_pos + 1..];
            Some(
                self.properties
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| default.to_string()),
            )
        } else {
            self.properties.get(inner).cloned()
        }
    }

    /// Return a plain snapshot (cloned HashMap) for passing into supplier closures.
    pub fn as_map(&self) -> HashMap<String, String> {
        self.properties.clone()
    }
}
