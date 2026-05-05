use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn parse_value(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let data = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("env.parse requires a string"))?;

    let mut map = HashMap::new();

    for line in data.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split on the first '=' only
        let eq_pos = line
            .find('=')
            .ok_or_else(|| CorvoError::parsing(format!("invalid .env line: {}", line)))?;

        let key = line[..eq_pos].trim().to_string();
        if key.is_empty() {
            return Err(CorvoError::parsing(format!("invalid .env line: {}", line)));
        }

        let raw_value = line[eq_pos + 1..].trim();

        // Strip surrounding quotes (single or double) if present
        let value = if raw_value.len() >= 2
            && ((raw_value.starts_with('"') && raw_value.ends_with('"'))
                || (raw_value.starts_with('\'') && raw_value.ends_with('\'')))
        {
            raw_value[1..raw_value.len() - 1].to_string()
        } else {
            raw_value.to_string()
        };

        map.insert(key, Value::String(value));
    }

    Ok(Value::Map(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_named_args() -> HashMap<String, Value> {
        HashMap::new()
    }

    #[test]
    fn test_parse_simple_key_value() {
        let input = "KEY=value";
        let args = vec![Value::String(input.to_string())];
        let result = parse_value(&args, &empty_named_args()).unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(m.get("KEY").unwrap(), &Value::String("value".to_string()));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_double_quoted_value() {
        let input = r#"KEY="hello world""#;
        let args = vec![Value::String(input.to_string())];
        let result = parse_value(&args, &empty_named_args()).unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(
                    m.get("KEY").unwrap(),
                    &Value::String("hello world".to_string())
                );
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_single_quoted_value() {
        let input = "KEY='hello world'";
        let args = vec![Value::String(input.to_string())];
        let result = parse_value(&args, &empty_named_args()).unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(
                    m.get("KEY").unwrap(),
                    &Value::String("hello world".to_string())
                );
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_skips_comments() {
        let input = "# this is a comment\nKEY=value";
        let args = vec![Value::String(input.to_string())];
        let result = parse_value(&args, &empty_named_args()).unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(m.len(), 1);
                assert_eq!(m.get("KEY").unwrap(), &Value::String("value".to_string()));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_skips_empty_lines() {
        let input = "\nKEY=value\n\nOTHER=foo\n";
        let args = vec![Value::String(input.to_string())];
        let result = parse_value(&args, &empty_named_args()).unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(m.len(), 2);
                assert_eq!(m.get("KEY").unwrap(), &Value::String("value".to_string()));
                assert_eq!(m.get("OTHER").unwrap(), &Value::String("foo".to_string()));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_multiple_entries() {
        let input = "DB_HOST=localhost\nDB_PORT=5432\nDB_NAME=mydb";
        let args = vec![Value::String(input.to_string())];
        let result = parse_value(&args, &empty_named_args()).unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(m.len(), 3);
                assert_eq!(
                    m.get("DB_HOST").unwrap(),
                    &Value::String("localhost".to_string())
                );
                assert_eq!(
                    m.get("DB_PORT").unwrap(),
                    &Value::String("5432".to_string())
                );
                assert_eq!(
                    m.get("DB_NAME").unwrap(),
                    &Value::String("mydb".to_string())
                );
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_value_with_equals_sign() {
        let input = r#"KEY="val=ue""#;
        let args = vec![Value::String(input.to_string())];
        let result = parse_value(&args, &empty_named_args()).unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(m.get("KEY").unwrap(), &Value::String("val=ue".to_string()));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_empty_value() {
        let input = "KEY=";
        let args = vec![Value::String(input.to_string())];
        let result = parse_value(&args, &empty_named_args()).unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(m.get("KEY").unwrap(), &Value::String("".to_string()));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_empty_string() {
        let args = vec![Value::String("".to_string())];
        let result = parse_value(&args, &empty_named_args()).unwrap();
        match result {
            Value::Map(m) => assert!(m.is_empty()),
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_no_args() {
        assert!(parse_value(&[], &empty_named_args()).is_err());
    }

    #[test]
    fn test_parse_invalid_line() {
        let input = "INVALID_LINE_WITHOUT_EQUALS";
        let args = vec![Value::String(input.to_string())];
        assert!(parse_value(&args, &empty_named_args()).is_err());
    }
}

#[macro_export]
macro_rules! env_parse {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("env.parse", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("env.parse", &[$($arg),*], &$kwargs, $state)
    };
}
