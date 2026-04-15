use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn parse_value(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let data = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("json.parse requires a string"))?;

    let parsed: serde_json::Value =
        serde_json::from_str(data).map_err(|e| CorvoError::parsing(e.to_string()))?;

    json_to_value(&parsed)
}

pub fn stringify(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let value = args
        .first()
        .ok_or_else(|| CorvoError::invalid_argument("json.stringify requires a value"))?;

    let json_value = value_to_json(value)?;
    let json_string = serde_json::to_string_pretty(&json_value)
        .map_err(|e| CorvoError::runtime(e.to_string()))?;

    Ok(Value::String(json_string))
}

pub fn json_to_value(json: &serde_json::Value) -> CorvoResult<Value> {
    match json {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(*b)),
        serde_json::Value::Number(n) => Ok(Value::Number(n.as_f64().unwrap_or(0.0))),
        serde_json::Value::String(s) => Ok(Value::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let values = arr
                .iter()
                .map(json_to_value)
                .collect::<CorvoResult<Vec<_>>>()?;
            Ok(Value::List(values))
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_value(v)?);
            }
            Ok(Value::Map(map))
        }
    }
}

pub fn value_to_json(value: &Value) -> CorvoResult<serde_json::Value> {
    match value {
        Value::Null => Ok(serde_json::Value::Null),
        Value::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Number(n) => serde_json::Number::from_f64(*n)
            .map(serde_json::Value::Number)
            .ok_or_else(|| CorvoError::runtime("Invalid number".to_string())),
        Value::String(s) => Ok(serde_json::Value::String(s.clone())),
        Value::List(arr) => {
            let values = arr
                .iter()
                .map(value_to_json)
                .collect::<CorvoResult<Vec<_>>>()?;
            Ok(serde_json::Value::Array(values))
        }
        Value::Map(map) => {
            let obj = map
                .iter()
                .map(|(k, v)| Ok((k.clone(), value_to_json(v)?)))
                .collect::<CorvoResult<serde_json::Map<String, serde_json::Value>>>()?;
            Ok(serde_json::Value::Object(obj))
        }
        Value::Regex(pattern, flags) => {
            Ok(serde_json::Value::String(format!("/{}/{}", pattern, flags)))
        }
        Value::Procedure(_) | Value::NativeProcedure { .. } => Err(CorvoError::r#type(
            "procedures cannot be serialized to JSON",
        )),
        Value::Shared(_) => Err(CorvoError::r#type(
            "shared values cannot be serialized to JSON",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_args() -> HashMap<String, Value> {
        HashMap::new()
    }

    #[test]
    fn test_parse_number() {
        let args = vec![Value::String("42".to_string())];
        assert_eq!(
            parse_value(&args, &empty_args()).unwrap(),
            Value::Number(42.0)
        );
    }

    #[test]
    fn test_parse_string() {
        let args = vec![Value::String(r#""hello""#.to_string())];
        assert_eq!(
            parse_value(&args, &empty_args()).unwrap(),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_parse_bool() {
        let args = vec![Value::String("true".to_string())];
        assert_eq!(
            parse_value(&args, &empty_args()).unwrap(),
            Value::Boolean(true)
        );
    }

    #[test]
    fn test_parse_null() {
        let args = vec![Value::String("null".to_string())];
        assert_eq!(parse_value(&args, &empty_args()).unwrap(), Value::Null);
    }

    #[test]
    fn test_parse_array() {
        let args = vec![Value::String("[1, 2, 3]".to_string())];
        let result = parse_value(&args, &empty_args()).unwrap();
        match result {
            Value::List(items) => assert_eq!(items.len(), 3),
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_parse_object() {
        let args = vec![Value::String(r#"{"key": "value"}"#.to_string())];
        let result = parse_value(&args, &empty_args()).unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(m.get("key").unwrap(), &Value::String("value".to_string()));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_parse_invalid() {
        let args = vec![Value::String("not json".to_string())];
        assert!(parse_value(&args, &empty_args()).is_err());
    }

    #[test]
    fn test_stringify_number() {
        let args = vec![Value::Number(42.0)];
        let result = stringify(&args, &empty_args()).unwrap();
        match result {
            Value::String(s) => assert_eq!(s, "42.0"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_stringify_string() {
        let args = vec![Value::String("hello".to_string())];
        let result = stringify(&args, &empty_args()).unwrap();
        match result {
            Value::String(s) => assert_eq!(s, r#""hello""#),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_roundtrip() {
        let original = r#"{"name":"test","value":42}"#;
        let args = vec![Value::String(original.to_string())];
        let parsed = parse_value(&args, &empty_args()).unwrap();
        let stringified = stringify(&[parsed], &empty_args()).unwrap();
        // Parse again to verify
        let reparsed = parse_value(&[stringified], &empty_args()).unwrap();
        match reparsed {
            Value::Map(m) => {
                assert_eq!(m.get("name").unwrap(), &Value::String("test".to_string()));
                assert_eq!(m.get("value").unwrap(), &Value::Number(42.0));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_stringify_no_args() {
        assert!(stringify(&[], &empty_args()).is_err());
    }
}
