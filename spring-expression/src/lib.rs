pub mod ast;
pub mod evaluator;
pub mod parser;

pub use evaluator::spel_evaluator::Value;

use evaluator::SpelEvaluator;
use parser::SpelParser;
use std::collections::HashMap;

/// Evaluate a **SpEL** expression string against an env map.
///
/// `expr` should be the raw expression **without** the surrounding `#{...}`
/// wrapper (those are stripped by the `#[Value]` macro before calling here).
///
/// Returns the string representation of the computed value, ready for
/// `str::parse()` into the target field type.
///
/// # Examples
/// ```ignore
/// let mut env = HashMap::new();
/// env.insert("server.port".into(), "8080".into());
///
/// assert_eq!(eval("${server.port:9090} * 2", &env).unwrap(), "16160");
/// assert_eq!(eval("'hello'.toUpperCase()", &env).unwrap(), "HELLO");
/// assert_eq!(eval("3 > 2 ? 'yes' : 'no'", &env).unwrap(), "yes");
/// ```
pub fn eval(expr: &str, env: &HashMap<String, String>) -> Result<String, String> {
    let mut parser = SpelParser::new(expr);
    let ast = parser.parse()?;
    let evaluator = SpelEvaluator::new(env);
    evaluator.eval(&ast).map(|v| v.to_string_repr())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_from(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_literals() {
        let env = HashMap::new();
        assert_eq!(eval("42", &env).unwrap(), "42");
        assert_eq!(eval("3.14", &env).unwrap(), "3.14");
        assert_eq!(eval("true", &env).unwrap(), "true");
        assert_eq!(eval("false", &env).unwrap(), "false");
        assert_eq!(eval("'hi'", &env).unwrap(), "hi");
    }

    #[test]
    fn test_arithmetic() {
        let env = HashMap::new();
        assert_eq!(eval("2 + 3", &env).unwrap(), "5");
        assert_eq!(eval("10 - 4", &env).unwrap(), "6");
        assert_eq!(eval("3 * 4", &env).unwrap(), "12");
        assert_eq!(eval("10 / 2", &env).unwrap(), "5");
        assert_eq!(eval("10 % 3", &env).unwrap(), "1");
        assert_eq!(eval("2 + 3 * 4", &env).unwrap(), "14"); // precedence
    }

    #[test]
    fn test_comparison() {
        let env = HashMap::new();
        assert_eq!(eval("3 > 2", &env).unwrap(), "true");
        assert_eq!(eval("2 > 3", &env).unwrap(), "false");
        assert_eq!(eval("2 == 2", &env).unwrap(), "true");
        assert_eq!(eval("2 != 3", &env).unwrap(), "true");
    }

    #[test]
    fn test_logical() {
        let env = HashMap::new();
        assert_eq!(eval("true && false", &env).unwrap(), "false");
        assert_eq!(eval("true || false", &env).unwrap(), "true");
        assert_eq!(eval("!true", &env).unwrap(), "false");
    }

    #[test]
    fn test_ternary() {
        let env = HashMap::new();
        assert_eq!(eval("3 > 2 ? 'yes' : 'no'", &env).unwrap(), "yes");
        assert_eq!(eval("2 > 3 ? 'yes' : 'no'", &env).unwrap(), "no");
        assert_eq!(eval("true ? 1 + 1 : 0", &env).unwrap(), "2");
    }

    #[test]
    fn test_property() {
        let env = env_from(&[("server.port", "9090")]);
        assert_eq!(eval("${server.port:8080}", &env).unwrap(), "9090");
        assert_eq!(
            eval("${missing.key:default_val}", &env).unwrap(),
            "default_val"
        );
    }

    #[test]
    fn test_string_methods() {
        let env = HashMap::new();
        assert_eq!(eval("'hello'.toUpperCase()", &env).unwrap(), "HELLO");
        assert_eq!(eval("'WORLD'.toLowerCase()", &env).unwrap(), "world");
        assert_eq!(eval("'hello'.length()", &env).unwrap(), "5");
        assert_eq!(eval("'  hi  '.trim()", &env).unwrap(), "hi");
        assert_eq!(eval("'hello'.contains('ell')", &env).unwrap(), "true");
    }

    #[test]
    fn test_combined() {
        let env = env_from(&[("app.name", "rust-spring")]);
        // ${key} + string concat
        assert_eq!(
            eval("${app.name:unknown} + '-v2'", &env).unwrap(),
            "rust-spring-v2"
        );
        // ternary with property
        assert_eq!(
            eval("${app.name:x} == 'rust-spring' ? 'ok' : 'fail'", &env).unwrap(),
            "ok"
        );
    }
}
