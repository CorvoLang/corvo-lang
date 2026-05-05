use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn parse_value(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let data = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("csv.parse requires a string"))?;

    let delimiter = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.chars().next().unwrap_or(','))
        .unwrap_or(',');

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter as u8)
        .has_headers(true)
        .from_reader(data.as_bytes());

    let mut result = Vec::new();

    for record in reader.records() {
        let record = record.map_err(|e| CorvoError::parsing(e.to_string()))?;
        let row: Vec<Value> = record
            .iter()
            .map(|s| Value::String(s.to_string()))
            .collect();
        result.push(Value::List(row));
    }

    Ok(Value::List(result))
}

#[macro_export]
macro_rules! csv_parse {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("csv.parse", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("csv.parse", &[$($arg),*], &$kwargs, $state)
    };
}
