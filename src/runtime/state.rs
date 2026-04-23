use super::tcp_registry::TcpRegistry;
use crate::type_system::Value;
use crate::CorvoError;
use std::collections::HashMap;

/// Holds the current state of a Corvo program during execution.
///
/// The `RuntimeState` tracks normal variables, static variables (which persist
/// and can be captured at compile time), the arguments passed to the script,
/// and the registry of active TCP connections.
#[derive(Debug)]
pub struct RuntimeState {
    vars: HashMap<String, Value>,
    statics: HashMap<String, Value>,
    /// Arguments passed to the Corvo program (after the script path when using
    /// the interpreter, or after the executable when running a compiled binary).
    script_argv: Vec<String>,
    pub(crate) tcp: TcpRegistry,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            statics: HashMap::new(),
            script_argv: Vec::new(),
            tcp: TcpRegistry::new(),
        }
    }

    pub(crate) fn tcp(&self) -> &TcpRegistry {
        &self.tcp
    }

    pub fn set_script_argv(&mut self, argv: Vec<String>) {
        self.script_argv = argv;
    }

    pub fn script_argv(&self) -> &[String] {
        &self.script_argv
    }

    // --- Variable Operations ---

    pub fn var_get(&self, name: &str) -> Result<Value, CorvoError> {
        self.vars
            .get(name)
            .cloned()
            .ok_or_else(|| CorvoError::variable_not_found(name))
    }

    pub fn var_set(&mut self, name: String, value: Value) {
        self.vars.insert(name, value);
    }

    pub fn var_remove(&mut self, name: &str) -> Option<Value> {
        self.vars.remove(name)
    }

    pub fn has_var(&self, name: &str) -> bool {
        self.vars.contains_key(name)
    }

    pub fn var_keys(&self) -> Vec<String> {
        self.vars.keys().cloned().collect()
    }

    pub fn var_count(&self) -> usize {
        self.vars.len()
    }

    pub fn clear_vars(&mut self) {
        self.vars.clear();
    }

    // --- Static Variable Operations ---

    pub fn static_get(&self, name: &str) -> Result<Value, CorvoError> {
        self.statics
            .get(name)
            .cloned()
            .ok_or_else(|| CorvoError::static_not_found(name))
    }

    pub fn static_set(&mut self, name: String, value: Value) {
        self.statics.insert(name, value);
    }

    pub fn static_remove(&mut self, name: &str) -> Option<Value> {
        self.statics.remove(name)
    }

    pub fn has_static(&self, name: &str) -> bool {
        self.statics.contains_key(name)
    }

    pub fn static_keys(&self) -> Vec<String> {
        self.statics.keys().cloned().collect()
    }

    pub fn static_count(&self) -> usize {
        self.statics.len()
    }

    pub fn clear_statics(&mut self) {
        self.statics.clear();
    }

    // --- Combined Operations ---

    pub fn is_empty(&self) -> bool {
        self.vars.is_empty() && self.statics.is_empty()
    }

    pub fn total_count(&self) -> usize {
        self.vars.len() + self.statics.len()
    }

    pub fn statics_snapshot(&self) -> HashMap<String, Value> {
        self.statics.clone()
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for RuntimeState {
    fn clone(&self) -> Self {
        Self {
            vars: self.vars.clone(),
            statics: self.statics.clone(),
            script_argv: self.script_argv.clone(),
            // Sockets are not duplicated; cloned state starts with no live handles.
            tcp: TcpRegistry::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_set_get() {
        let mut state = RuntimeState::new();
        state.var_set("x".to_string(), Value::Number(42.0));
        assert_eq!(state.var_get("x").unwrap(), Value::Number(42.0));
    }

    #[test]
    fn test_var_not_found() {
        let state = RuntimeState::new();
        let err = state.var_get("nonexistent").unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("nonexistent"));
    }

    #[test]
    fn test_var_overwrite() {
        let mut state = RuntimeState::new();
        state.var_set("x".to_string(), Value::Number(1.0));
        state.var_set("x".to_string(), Value::Number(2.0));
        assert_eq!(state.var_get("x").unwrap(), Value::Number(2.0));
    }

    #[test]
    fn test_var_remove() {
        let mut state = RuntimeState::new();
        state.var_set("x".to_string(), Value::Number(1.0));
        let removed = state.var_remove("x");
        assert_eq!(removed, Some(Value::Number(1.0)));
        assert!(!state.has_var("x"));
    }

    #[test]
    fn test_var_remove_nonexistent() {
        let mut state = RuntimeState::new();
        assert_eq!(state.var_remove("missing"), None);
    }

    #[test]
    fn test_has_var() {
        let mut state = RuntimeState::new();
        assert!(!state.has_var("x"));
        state.var_set("x".to_string(), Value::Null);
        assert!(state.has_var("x"));
    }

    #[test]
    fn test_var_keys() {
        let mut state = RuntimeState::new();
        state.var_set("a".to_string(), Value::Number(1.0));
        state.var_set("b".to_string(), Value::Number(2.0));
        let mut keys = state.var_keys();
        keys.sort();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn test_var_count() {
        let mut state = RuntimeState::new();
        assert_eq!(state.var_count(), 0);
        state.var_set("x".to_string(), Value::Null);
        state.var_set("y".to_string(), Value::Null);
        assert_eq!(state.var_count(), 2);
    }

    #[test]
    fn test_clear_vars() {
        let mut state = RuntimeState::new();
        state.var_set("x".to_string(), Value::Number(1.0));
        state.var_set("y".to_string(), Value::Number(2.0));
        state.clear_vars();
        assert_eq!(state.var_count(), 0);
        assert!(!state.has_var("x"));
    }

    // --- Static Tests ---

    #[test]
    fn test_static_set_get() {
        let mut state = RuntimeState::new();
        state.static_set("PI".to_string(), Value::Number(std::f64::consts::PI));
        assert_eq!(
            state.static_get("PI").unwrap(),
            Value::Number(std::f64::consts::PI)
        );
    }

    #[test]
    fn test_static_not_found() {
        let state = RuntimeState::new();
        let err = state.static_get("missing").unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("missing"));
    }

    #[test]
    fn test_static_overwrite() {
        let mut state = RuntimeState::new();
        state.static_set("X".to_string(), Value::Number(1.0));
        state.static_set("X".to_string(), Value::Number(2.0));
        assert_eq!(state.static_get("X").unwrap(), Value::Number(2.0));
    }

    #[test]
    fn test_static_remove() {
        let mut state = RuntimeState::new();
        state.static_set("X".to_string(), Value::Number(1.0));
        let removed = state.static_remove("X");
        assert_eq!(removed, Some(Value::Number(1.0)));
        assert!(!state.has_static("X"));
    }

    #[test]
    fn test_has_static() {
        let mut state = RuntimeState::new();
        assert!(!state.has_static("X"));
        state.static_set("X".to_string(), Value::Null);
        assert!(state.has_static("X"));
    }

    #[test]
    fn test_static_keys() {
        let mut state = RuntimeState::new();
        state.static_set("A".to_string(), Value::Number(1.0));
        state.static_set("B".to_string(), Value::Number(2.0));
        let mut keys = state.static_keys();
        keys.sort();
        assert_eq!(keys, vec!["A", "B"]);
    }

    #[test]
    fn test_static_count() {
        let mut state = RuntimeState::new();
        assert_eq!(state.static_count(), 0);
        state.static_set("X".to_string(), Value::Null);
        assert_eq!(state.static_count(), 1);
    }

    #[test]
    fn test_clear_statics() {
        let mut state = RuntimeState::new();
        state.static_set("X".to_string(), Value::Number(1.0));
        state.clear_statics();
        assert_eq!(state.static_count(), 0);
    }

    // --- Combined Tests ---

    #[test]
    fn test_is_empty() {
        let state = RuntimeState::new();
        assert!(state.is_empty());

        let mut state = RuntimeState::new();
        state.var_set("x".to_string(), Value::Null);
        assert!(!state.is_empty());
    }

    #[test]
    fn test_total_count() {
        let mut state = RuntimeState::new();
        assert_eq!(state.total_count(), 0);
        state.var_set("x".to_string(), Value::Null);
        state.static_set("Y".to_string(), Value::Null);
        assert_eq!(state.total_count(), 2);
    }

    #[test]
    fn test_default() {
        let state = RuntimeState::default();
        assert!(state.is_empty());
    }

    #[test]
    fn test_clone() {
        let mut state = RuntimeState::new();
        state.var_set("x".to_string(), Value::Number(42.0));
        let cloned = state.clone();
        assert_eq!(cloned.var_get("x").unwrap(), Value::Number(42.0));
    }

    #[test]
    fn test_var_static_independent() {
        let mut state = RuntimeState::new();
        state.var_set("x".to_string(), Value::Number(1.0));
        state.static_set("x".to_string(), Value::Number(2.0));
        assert_eq!(state.var_get("x").unwrap(), Value::Number(1.0));
        assert_eq!(state.static_get("x").unwrap(), Value::Number(2.0));
    }

    #[test]
    fn test_script_argv_default_empty() {
        let state = RuntimeState::new();
        assert!(state.script_argv().is_empty());
    }

    #[test]
    fn test_set_script_argv() {
        let mut state = RuntimeState::new();
        state.set_script_argv(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(state.script_argv(), &["a", "b"]);
    }
}
