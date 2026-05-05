use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use handlebars::Handlebars;
use std::collections::HashMap;

/// Render a Handlebars template string with the given data map.
///
/// Arguments:
///   0 - template_string: string  — the Handlebars template
///   1 - data: map                — key/value context passed to the template
///
/// Returns: string
pub fn render(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() < 2 {
        return Err(CorvoError::invalid_argument(
            "template.render requires a template string and a data map",
        ));
    }

    let template_str = args[0]
        .as_string()
        .ok_or_else(|| CorvoError::r#type("template.render: first argument must be a string"))?;

    let data_json = crate::standard_lib::json::value_to_json(&args[1])?;

    let mut reg = Handlebars::new();
    reg.set_strict_mode(false);

    let rendered = reg
        .render_template(template_str, &data_json)
        .map_err(|e| CorvoError::runtime(format!("template.render error: {e}")))?;

    Ok(Value::String(rendered))
}

/// Render a Handlebars template loaded from a file with the given data map.
///
/// Arguments:
///   0 - path: string  — path to the template file
///   1 - data: map     — key/value context passed to the template
///
/// Returns: string
pub fn render_file(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() < 2 {
        return Err(CorvoError::invalid_argument(
            "template.render_file requires a file path and a data map",
        ));
    }

    let path = args[0].as_string().ok_or_else(|| {
        CorvoError::r#type("template.render_file: first argument must be a string")
    })?;

    let template_str = std::fs::read_to_string(path).map_err(|e| {
        CorvoError::runtime(format!("template.render_file: could not read file: {e}"))
    })?;

    let data_json = crate::standard_lib::json::value_to_json(&args[1])?;

    let mut reg = Handlebars::new();
    reg.set_strict_mode(false);

    let rendered = reg
        .render_template(&template_str, &data_json)
        .map_err(|e| CorvoError::runtime(format!("template.render_file error: {e}")))?;

    Ok(Value::String(rendered))
}

#[macro_export]
macro_rules! template_render {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("template.render", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("template.render", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! template_render_file {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("template.render_file", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("template.render_file", &[$($arg),*], &$kwargs, $state)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_named() -> HashMap<String, Value> {
        HashMap::new()
    }

    fn make_map(pairs: &[(&str, &str)]) -> Value {
        let mut m = HashMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), Value::String(v.to_string()));
        }
        Value::Map(m)
    }

    #[test]
    fn test_render_simple() {
        let args = vec![
            Value::String("Hello, {{name}}!".to_string()),
            make_map(&[("name", "Corvo")]),
        ];
        let result = render(&args, &empty_named()).unwrap();
        assert_eq!(result, Value::String("Hello, Corvo!".to_string()));
    }

    #[test]
    fn test_render_multiple_vars() {
        let args = vec![
            Value::String("{{greeting}}, {{name}}! You are {{age}} years old.".to_string()),
            make_map(&[("greeting", "Hi"), ("name", "World"), ("age", "42")]),
        ];
        let result = render(&args, &empty_named()).unwrap();
        assert_eq!(
            result,
            Value::String("Hi, World! You are 42 years old.".to_string())
        );
    }

    #[test]
    fn test_render_missing_key_is_empty() {
        // Without strict mode, missing keys render as empty string
        let args = vec![
            Value::String("Hello, {{missing}}!".to_string()),
            make_map(&[]),
        ];
        let result = render(&args, &empty_named()).unwrap();
        assert_eq!(result, Value::String("Hello, !".to_string()));
    }

    #[test]
    fn test_render_no_args() {
        assert!(render(&[], &empty_named()).is_err());
    }

    #[test]
    fn test_render_one_arg() {
        let args = vec![Value::String("Hello".to_string())];
        assert!(render(&args, &empty_named()).is_err());
    }

    #[test]
    fn test_render_non_string_template() {
        let args = vec![Value::Number(42.0), make_map(&[])];
        assert!(render(&args, &empty_named()).is_err());
    }

    #[test]
    fn test_render_file_no_args() {
        assert!(render_file(&[], &empty_named()).is_err());
    }

    #[test]
    fn test_render_file_missing_file() {
        let args = vec![
            Value::String("/nonexistent/path/template.hbs".to_string()),
            make_map(&[("name", "test")]),
        ];
        assert!(render_file(&args, &empty_named()).is_err());
    }
}
