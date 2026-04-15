use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn parse_value(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let data = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("yaml.parse requires a string"))?;

    let parsed: serde_yaml::Value =
        serde_yaml::from_str(data).map_err(|e| CorvoError::parsing(e.to_string()))?;

    yaml_value_to_value(&parsed)
}

pub fn stringify(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let value = args
        .first()
        .ok_or_else(|| CorvoError::invalid_argument("yaml.stringify requires a value"))?;

    let yaml_value = value_to_yaml_value(value)?;
    let yaml_string =
        serde_yaml::to_string(&yaml_value).map_err(|e| CorvoError::runtime(e.to_string()))?;

    Ok(Value::String(yaml_string))
}

fn yaml_value_to_value(yaml: &serde_yaml::Value) -> CorvoResult<Value> {
    match yaml {
        serde_yaml::Value::Null => Ok(Value::Null),
        serde_yaml::Value::Bool(b) => Ok(Value::Boolean(*b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Number(i as f64))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Number(f))
            } else {
                Ok(Value::Null)
            }
        }
        serde_yaml::Value::String(s) => Ok(Value::String(s.clone())),
        serde_yaml::Value::Sequence(arr) => {
            let values = arr
                .iter()
                .map(yaml_value_to_value)
                .collect::<CorvoResult<Vec<_>>>()?;
            Ok(Value::List(values))
        }
        serde_yaml::Value::Mapping(map) => {
            let mut result = HashMap::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                result.insert(key, yaml_value_to_value(v)?);
            }
            Ok(Value::Map(result))
        }
        serde_yaml::Value::Tagged(tagged) => yaml_value_to_value(&tagged.value),
    }
}

fn value_to_yaml_value(value: &Value) -> CorvoResult<serde_yaml::Value> {
    match value {
        Value::Null => Ok(serde_yaml::Value::Null),
        Value::Boolean(b) => Ok(serde_yaml::Value::Bool(*b)),
        Value::Number(n) => Ok(serde_yaml::Value::Number(serde_yaml::Number::from(
            *n as i64,
        ))),
        Value::String(s) => Ok(serde_yaml::Value::String(s.clone())),
        Value::List(arr) => {
            let values = arr
                .iter()
                .map(value_to_yaml_value)
                .collect::<CorvoResult<Vec<_>>>()?;
            Ok(serde_yaml::Value::Sequence(values))
        }
        Value::Map(map) => {
            let mut result = serde_yaml::Mapping::new();
            for (k, v) in map {
                let yaml_key = serde_yaml::Value::String(k.clone());
                result.insert(yaml_key, value_to_yaml_value(v)?);
            }
            Ok(serde_yaml::Value::Mapping(result))
        }
        Value::Regex(pattern, flags) => {
            Ok(serde_yaml::Value::String(format!("/{}/{}", pattern, flags)))
        }
        Value::Procedure(_) | Value::NativeProcedure { .. } => Err(CorvoError::r#type(
            "procedures cannot be serialized to YAML",
        )),
        Value::Shared(_) => Err(CorvoError::r#type(
            "shared values cannot be serialized to YAML",
        )),
    }
}
