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

    pub fn vars_snapshot(&self) -> HashMap<String, Value> {
        self.vars.clone()
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

    #[test]
    fn test_vars_snapshot() {
        let mut state = RuntimeState::default();
        state.var_set("x".to_string(), Value::Number(1.0));
        state.var_set("y".to_string(), Value::Number(2.0));
        let mut snap = state.vars_snapshot();
        // Snapshot reflects current state
        assert_eq!(snap.get("x"), Some(&Value::Number(1.0)));
        assert_eq!(snap.get("y"), Some(&Value::Number(2.0)));
        // Mutating the snapshot does not affect the original
        snap.insert("x".to_string(), Value::Number(99.0));
        assert_eq!(state.var_get("x").unwrap(), Value::Number(1.0));
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

#[macro_export]
macro_rules! var_get {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("var.get", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("var.get", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! var_set {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("var.set", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("var.set", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! static_get {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("static.get", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("static.get", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! static_set {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("static.set", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("static.set", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! corvo_method_call {
    ($state:expr, $target:expr, $method:expr, $args:expr, $outer_names:expr) => {{
        let mut t = $target;
        #[allow(clippy::eq_op)]
        if $method == "call" && matches!(t, $crate::type_system::Value::NativeProcedure { .. }) {
            if let $crate::type_system::Value::NativeProcedure { params: p_names, callback: p } = t {
                let saved: Vec<Option<$crate::type_system::Value>> = p_names.iter().map(|p| $state.var_remove(p)).collect();
                let res = (p)(&$args, $state)?;
                for (i, param) in p_names.iter().enumerate() {
                    if let Some(Some(outer_name)) = $outer_names.get(i) {
                        let updated = $state.var_get(param.as_str()).unwrap_or($crate::type_system::Value::Null);
                        $state.var_set(outer_name.clone(), updated);
                    }
                    $state.var_remove(param.as_str());
                    if let Some(prev) = saved[i].clone() {
                        $state.var_set(param.to_string(), prev);
                    }
                }
                res
            } else { unreachable!() }
        } else {
            let mut a = vec![t.clone()];
            a.extend($args);
            let ns = match &t {
                $crate::type_system::Value::String(_) => "string",
                $crate::type_system::Value::Number(_) => "number",
                $crate::type_system::Value::List(_) => "list",
                $crate::type_system::Value::Map(_) => "map",
                $crate::type_system::Value::Regex(_, _) => "re",
                $crate::type_system::Value::DatabasePool(_) => "db",
                $crate::type_system::Value::AmqpConnection(_) => "amqp",
                $crate::type_system::Value::Procedure(_) | $crate::type_system::Value::NativeProcedure { .. } => "procedure",
                _ => return Err($crate::CorvoError::r#type("method call error")),
            };
            $crate::standard_lib::call(&format!("{}.{}", ns, $method), &a, &std::collections::HashMap::new(), $state)?
        }
    }};
}

#[macro_export]
macro_rules! corvo_browse {
    ($state:expr, $iter_val:expr, $key:expr, $value:expr, $body:expr) => {
        match $iter_val {
            $crate::type_system::Value::List(list) => {
                for (i, item) in list.iter().enumerate() {
                    $state.var_set($key.to_string(), $crate::type_system::Value::Number(i as f64));
                    $state.var_set($value.to_string(), item.clone());
                    $body
                }
            }
            $crate::type_system::Value::Map(map) => {
                let mut entries: Vec<_> = map.into_iter().collect();
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                for (k, v) in entries {
                    $state.var_set($key.to_string(), $crate::type_system::Value::String(k));
                    $state.var_set($value.to_string(), v);
                    $body
                }
            }
            _ => return Err($crate::CorvoError::r#type("browse only works on lists and maps")),
        }
    }
}

#[macro_export]
macro_rules! corvo_assert {
    (eq, $v0:expr, $v1:expr) => {
        if $v0 != $v1 { return Err($crate::CorvoError::assertion(format!("{} != {}", $v0, $v1))); }
    };
    (neq, $v0:expr, $v1:expr) => {
        if $v0 == $v1 { return Err($crate::CorvoError::assertion(format!("{} == {}", $v0, $v1))); }
    };
    (gt, $v0:expr, $v1:expr) => {
        if $v0.as_number().unwrap_or(0.0) <= $v1.as_number().unwrap_or(0.0) { return Err($crate::CorvoError::assertion(format!("{} <= {}", $v0, $v1))); }
    };
    (lt, $v0:expr, $v1:expr) => {
        if $v0.as_number().unwrap_or(0.0) >= $v1.as_number().unwrap_or(0.0) { return Err($crate::CorvoError::assertion(format!("{} >= {}", $v0, $v1))); }
    };
    (ge, $v0:expr, $v1:expr) => {
        if $v0.as_number().unwrap_or(0.0) < $v1.as_number().unwrap_or(0.0) { return Err($crate::CorvoError::assertion(format!("{} < {}", $v0, $v1))); }
    };
    (le, $v0:expr, $v1:expr) => {
        if $v0.as_number().unwrap_or(0.0) > $v1.as_number().unwrap_or(0.0) { return Err($crate::CorvoError::assertion(format!("{} > {}", $v0, $v1))); }
    };
    (match, $v0:expr, $v1:expr) => {{
        let pattern = $v0.as_string().ok_or_else(|| $crate::CorvoError::r#type("assert_match requires strings"))?;
        let target = $v1.as_string().ok_or_else(|| $crate::CorvoError::r#type("assert_match requires strings"))?;
        let re = regex::Regex::new(&pattern).map_err(|e| $crate::CorvoError::runtime(e.to_string()))?;
        if !re.is_match(&target) { return Err($crate::CorvoError::assertion(format!("{} does not match {}", $v0, $v1))); }
    }};
}

#[macro_export]
macro_rules! corvo_index {
    ($target:expr, $index:expr) => {
        match ($target, $index) {
            ($crate::type_system::Value::List(l), $crate::type_system::Value::Number(idx)) => {
                if !idx.is_finite() || idx < 0.0 || idx.fract() != 0.0 {
                    return Err($crate::CorvoError::runtime("Invalid index"));
                }
                l.get(idx as usize).cloned().ok_or_else(|| $crate::CorvoError::runtime("Index out of bounds"))?
            },
            ($crate::type_system::Value::Map(m), $crate::type_system::Value::String(key)) => m.get(&key).cloned().ok_or_else(|| $crate::CorvoError::runtime(format!("Key not found: {}", key)))?,
            _ => return Err($crate::CorvoError::r#type("index access error"))
        }
    }
}

#[macro_export]
macro_rules! corvo_slice {
    ($target:expr, $start:expr, $end:expr) => {
        match $target {
            $crate::type_system::Value::List(l) => {
                let start = $start.unwrap_or(0);
                let end = $end.unwrap_or(l.len());
                $crate::type_system::Value::List(l[start.min(l.len())..end.min(l.len())].to_vec())
            }
            $crate::type_system::Value::String(s) => {
                let start = $start.unwrap_or(0);
                let end = $end.unwrap_or(s.len());
                $crate::type_system::Value::String(s[start.min(s.len())..end.min(s.len())].to_string())
            }
            _ => return Err($crate::CorvoError::r#type("slice access error"))
        }
    }
}
