mod strverscmp;

pub mod ast;
pub mod compiler;
pub mod diagnostic;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod repl;
pub mod runtime;
pub mod span;
pub mod standard_lib;
pub mod type_system;

pub use error::{CorvoError, CorvoResult};
pub use runtime::RuntimeState;
pub use span::{Position, Span};

use crate::compiler::Evaluator;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::path::Path;

pub fn run_file(path: &Path) -> CorvoResult<()> {
    let source = std::fs::read_to_string(path).map_err(|e| CorvoError::io(e.to_string()))?;
    run_source(&source)
}

pub fn run_source(source: &str) -> CorvoResult<()> {
    let mut state = RuntimeState::new();
    run_source_with_state(source, &mut state)
}

/// Run source with a preset script argument vector (for `os.argv()` / `args.scan`).
pub fn run_source_with_script_argv(source: &str, script_argv: Vec<String>) -> CorvoResult<()> {
    let mut state = RuntimeState::new();
    state.set_script_argv(script_argv);
    run_source_with_state(source, &mut state)
}

pub fn run_source_with_state(source: &str, state: &mut RuntimeState) -> CorvoResult<()> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(
        tokens
            .into_iter()
            .filter(|t| !matches!(t.token_type, crate::lexer::token::TokenType::Comment(_)))
            .collect(),
    );
    let program = parser.parse()?;

    let mut evaluator = Evaluator::new();
    evaluator.run(&program, state)?;

    Ok(())
}

pub fn run_repl() {
    repl::run();
}

/// Decrypt and load static variables that were encrypted at compile time.
///
/// The `encrypted` bytes are XOR-decrypted with `key` and then deserialized
/// from a JSON object into the runtime state's static storage.  Storing
/// statics this way keeps all static keys and string values out of the
/// compiled binary as human-readable strings (they won't appear in `strings`
/// output).
pub fn load_statics_from_encrypted_bytes(state: &mut RuntimeState, encrypted: &[u8], key: &[u8]) {
    let decrypted: Vec<u8> = encrypted
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % key.len()])
        .collect();
    let json: serde_json::Value = serde_json::from_slice(&decrypted)
        .expect("failed to deserialize encrypted statics: data may be corrupted or was compiled with an incompatible version");
    if let serde_json::Value::Object(map) = json {
        for (k, v) in map {
            state.static_set(k, json_value_to_corvo_value(v));
        }
    }
}

/// Convert a `serde_json::Value` back to a corvo `Value`.
///
/// The special sentinel objects `{"__corvo_f64": "nan"}`,
/// `{"__corvo_f64": "inf"}`, and `{"__corvo_f64": "-inf"}` are decoded back
/// to their `f64` counterparts so that non-JSON-safe numbers survive the
/// round-trip through the encrypted statics blob.
fn json_value_to_corvo_value(json: serde_json::Value) -> type_system::Value {
    match json {
        serde_json::Value::Null => type_system::Value::Null,
        serde_json::Value::Bool(b) => type_system::Value::Boolean(b),
        serde_json::Value::Number(n) => type_system::Value::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => type_system::Value::String(s),
        serde_json::Value::Array(arr) => {
            type_system::Value::List(arr.into_iter().map(json_value_to_corvo_value).collect())
        }
        serde_json::Value::Object(obj) => {
            // Decode special NaN / Infinity sentinels.
            if obj.len() == 1 {
                if let Some(tag) = obj.get("__corvo_f64") {
                    match tag.as_str() {
                        Some("nan") => return type_system::Value::Number(f64::NAN),
                        Some("inf") => return type_system::Value::Number(f64::INFINITY),
                        Some("-inf") => return type_system::Value::Number(f64::NEG_INFINITY),
                        _ => {}
                    }
                }
                if let Some(serde_json::Value::Object(r)) = obj.get("__corvo_regex") {
                    let pattern = r
                        .get("pattern")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let flags = r
                        .get("flags")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    return type_system::Value::Regex(pattern, flags);
                }
            }
            let map = obj
                .into_iter()
                .map(|(k, v)| (k, json_value_to_corvo_value(v)))
                .collect();
            type_system::Value::Map(map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xor(data: &[u8], key: &[u8]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, &b)| b ^ key[i % key.len()])
            .collect()
    }

    #[test]
    fn test_load_statics_from_encrypted_bytes_strings() {
        let json = serde_json::json!({
            "DB_URL": "postgres://localhost/db",
            "APP_ENV": "production"
        });
        let json_bytes = serde_json::to_vec(&json).unwrap();
        let key = b"testkey1234567890testkey12345678";
        let encrypted = xor(&json_bytes, key);

        let mut state = RuntimeState::new();
        load_statics_from_encrypted_bytes(&mut state, &encrypted, key);

        assert_eq!(
            state.static_get("DB_URL").unwrap(),
            type_system::Value::String("postgres://localhost/db".to_string())
        );
        assert_eq!(
            state.static_get("APP_ENV").unwrap(),
            type_system::Value::String("production".to_string())
        );
    }

    #[test]
    fn test_load_statics_from_encrypted_bytes_mixed_types() {
        let json = serde_json::json!({
            "NUM": 42.0,
            "FLAG": true,
            "EMPTY": null
        });
        let json_bytes = serde_json::to_vec(&json).unwrap();
        let key = b"anotherkey123456789012345678901";
        let encrypted = xor(&json_bytes, key);

        let mut state = RuntimeState::new();
        load_statics_from_encrypted_bytes(&mut state, &encrypted, key);

        assert_eq!(
            state.static_get("NUM").unwrap(),
            type_system::Value::Number(42.0)
        );
        assert_eq!(
            state.static_get("FLAG").unwrap(),
            type_system::Value::Boolean(true)
        );
        assert_eq!(state.static_get("EMPTY").unwrap(), type_system::Value::Null);
    }

    #[test]
    fn test_load_statics_from_encrypted_bytes_nan_inf() {
        let json = serde_json::json!({
            "NAN_VAL": {"__corvo_f64": "nan"},
            "INF_VAL": {"__corvo_f64": "inf"},
            "NEG_INF": {"__corvo_f64": "-inf"}
        });
        let json_bytes = serde_json::to_vec(&json).unwrap();
        let key = b"somekey0987654321somekey0987654";
        let encrypted = xor(&json_bytes, key);

        let mut state = RuntimeState::new();
        load_statics_from_encrypted_bytes(&mut state, &encrypted, key);

        if let type_system::Value::Number(n) = state.static_get("NAN_VAL").unwrap() {
            assert!(n.is_nan());
        } else {
            panic!("expected NaN");
        }
        assert_eq!(
            state.static_get("INF_VAL").unwrap(),
            type_system::Value::Number(f64::INFINITY)
        );
        assert_eq!(
            state.static_get("NEG_INF").unwrap(),
            type_system::Value::Number(f64::NEG_INFINITY)
        );
    }

    #[test]
    fn test_load_statics_from_encrypted_bytes_empty() {
        let json = serde_json::json!({});
        let json_bytes = serde_json::to_vec(&json).unwrap();
        let key = b"key1234567890123456789012345678";
        let encrypted = xor(&json_bytes, key);

        let mut state = RuntimeState::new();
        load_statics_from_encrypted_bytes(&mut state, &encrypted, key);
        assert_eq!(state.static_count(), 0);
    }
}
