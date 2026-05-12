use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn parse_value(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let data = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("csv.parse requires a string"))?;

    let delimiter = match args.get(1).and_then(|v| v.as_string()) {
        Some(s) => {
            if s.len() != 1 {
                return Err(CorvoError::invalid_argument(
                    "csv.parse: delimiter must be a single byte (ASCII)",
                ));
            }
            s.as_bytes()[0]
        }
        None => b',',
    };

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn no_args() -> HashMap<String, Value> {
        HashMap::new()
    }

    #[test]
    fn parse_rejects_empty_delimiter() {
        let err = parse_value(
            &[
                Value::String("a:b:c\n".to_string()),
                Value::String("".to_string()),
            ],
            &no_args(),
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("delimiter must be a single byte (ASCII)"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_rejects_multi_character_delimiter() {
        let err = parse_value(
            &[
                Value::String("a::b::c\n".to_string()),
                Value::String("::".to_string()),
            ],
            &no_args(),
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("delimiter must be a single byte (ASCII)"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_with_single_char_delimiter_extracts_expected_field() {
        // First row is header; second row is the parsed record.
        let result = parse_value(
            &[
                Value::String("c1:c2:c3\nx:y:z\n".to_string()),
                Value::String(":".to_string()),
            ],
            &no_args(),
        )
        .unwrap();

        let rows = result.as_list().unwrap();
        assert_eq!(rows.len(), 1);
        let row = rows[0].as_list().unwrap();
        assert_eq!(row[1], Value::String("y".to_string()));
        assert_ne!(row[1], Value::Null);
    }

    /// Regression for issue #18: tightening the delimiter validation must not regress the
    /// "no delimiter argument" case, which has to keep defaulting to ASCII comma.
    #[test]
    fn parse_without_delimiter_defaults_to_comma() {
        let result =
            parse_value(&[Value::String("a,b,c\n1,2,3\n".to_string())], &no_args()).unwrap();

        let rows = result.as_list().unwrap();
        assert_eq!(rows.len(), 1);
        let row = rows[0].as_list().unwrap();
        assert_eq!(row.len(), 3);
        assert_eq!(row[0], Value::String("1".to_string()));
        assert_eq!(row[1], Value::String("2".to_string()));
        assert_eq!(row[2], Value::String("3".to_string()));
    }
}
