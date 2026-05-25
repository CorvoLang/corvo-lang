#[cfg(all(unix, feature = "stdlib-pex"))]
use crate::runtime::pex_registry::PexEntry;
use crate::runtime::pex_registry::{KIND_BASH, KIND_SESSION};
use crate::runtime::RuntimeState;
use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// Escape a single argument for `/bin/sh -c` style command strings used by rexpect.
fn shell_escape_unix(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "-._/:".contains(c))
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Spawn a subprocess in a PTY from an argv vector (used by `run_test`).
#[cfg(all(unix, feature = "stdlib-pex"))]
pub(crate) fn spawn_argv_for_test(
    argv: &[String],
    timeout_ms: u64,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    if argv.is_empty() {
        return Err(CorvoError::invalid_argument(
            "run_test spawn requires at least one argv element (executable path)",
        ));
    }
    let command = argv
        .iter()
        .map(|s| shell_escape_unix(s))
        .collect::<Vec<_>>()
        .join(" ");
    let session = rexpect::spawn(command.as_str(), Some(timeout_ms)).map_err(map_rexpect_err)?;
    let id = state.pex().insert_session(session);
    Ok(session_handle_map(KIND_SESSION, id, Some(command.as_str())))
}

#[cfg(not(all(unix, feature = "stdlib-pex")))]
pub(crate) fn spawn_argv_for_test(
    _argv: &[String],
    _timeout_ms: u64,
    _state: &RuntimeState,
) -> CorvoResult<Value> {
    Err(unix_only_error())
}

#[cfg(not(all(unix, feature = "stdlib-pex")))]
fn unix_only_error() -> CorvoError {
    CorvoError::runtime("pex.* is only available on Unix".to_string())
}

fn session_id_from_value(v: &Value) -> CorvoResult<u64> {
    let m = v.as_map().ok_or_else(|| {
        CorvoError::invalid_argument(
            "expected pex session handle map from pex.spawn / pex.spawn_bash",
        )
    })?;
    let kind = m
        .get("kind")
        .and_then(|x| x.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| CorvoError::invalid_argument("pex session handle missing string kind"))?;
    if kind != KIND_SESSION && kind != KIND_BASH {
        return Err(CorvoError::invalid_argument(
            "expected pex session handle (wrong kind)",
        ));
    }
    let id = m
        .get("id")
        .and_then(|x| x.as_number())
        .ok_or_else(|| CorvoError::invalid_argument("pex session handle missing numeric id"))?;
    if id < 0.0 || id.fract() != 0.0 {
        return Err(CorvoError::invalid_argument(
            "pex session handle id must be a non-negative integer",
        ));
    }
    Ok(id as u64)
}

fn session_handle_map(kind: &str, id: u64, command: Option<&str>) -> Value {
    let mut m = HashMap::new();
    m.insert("kind".to_string(), Value::String(kind.to_string()));
    m.insert("id".to_string(), Value::Number(id as f64));
    if let Some(cmd) = command {
        m.insert("command".to_string(), Value::String(cmd.to_string()));
    }
    Value::Map(m)
}

fn parse_timeout_ms(
    args: &[Value],
    named_args: &HashMap<String, Value>,
    positional_index: Option<usize>,
) -> CorvoResult<u64> {
    if let Some(v) = named_args.get("timeout_ms") {
        return v
            .as_number()
            .filter(|n| *n >= 0.0 && n.fract() == 0.0)
            .map(|n| n as u64)
            .ok_or_else(|| {
                CorvoError::invalid_argument("timeout_ms must be a non-negative integer")
            });
    }
    if let Some(idx) = positional_index {
        if let Some(v) = args.get(idx) {
            return v
                .as_number()
                .filter(|n| *n >= 0.0 && n.fract() == 0.0)
                .map(|n| n as u64)
                .ok_or_else(|| {
                    CorvoError::invalid_argument("timeout_ms must be a non-negative integer")
                });
        }
    }
    Ok(DEFAULT_TIMEOUT_MS)
}

#[cfg(all(unix, feature = "stdlib-pex"))]
fn map_rexpect_err(e: rexpect::error::Error) -> CorvoError {
    CorvoError::runtime(e.to_string())
}

#[cfg(all(unix, feature = "stdlib-pex"))]
fn with_pex_session<R>(
    state: &RuntimeState,
    handle: &Value,
    f: impl FnOnce(&mut PexEntry) -> CorvoResult<R>,
) -> CorvoResult<R> {
    let id = session_id_from_value(handle)?;
    state.pex().with_entry_mut(id, f)
}

#[cfg(all(unix, feature = "stdlib-pex"))]
fn with_pex_io<R>(
    entry: &mut PexEntry,
    f: impl FnOnce(&mut rexpect::session::PtySession) -> CorvoResult<R>,
) -> CorvoResult<R> {
    match entry {
        PexEntry::Session(s) => f(s),
        PexEntry::Repl(r) => f(&mut *r),
    }
}

/// `pex.spawn(command, [timeout_ms]) -> map` — spawn a command in a PTY (Unix only).
pub fn spawn(
    args: &[Value],
    named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let command = args
            .first()
            .and_then(|v| v.as_string())
            .ok_or_else(|| CorvoError::invalid_argument("pex.spawn requires a command string"))?;
        let timeout = parse_timeout_ms(args, named_args, Some(1))?;
        let session = rexpect::spawn(command.as_str(), Some(timeout)).map_err(map_rexpect_err)?;
        let id = state.pex().insert_session(session);
        Ok(session_handle_map(KIND_SESSION, id, Some(command.as_str())))
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, named_args, state);
        Err(unix_only_error())
    }
}

/// `pex.spawn_bash([timeout_ms]) -> map` — interactive bash REPL in a PTY (Unix only).
pub fn spawn_bash(
    args: &[Value],
    named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let timeout = parse_timeout_ms(args, named_args, Some(0))?;
        let repl = rexpect::spawn_bash(Some(timeout)).map_err(map_rexpect_err)?;
        let id = state.pex().insert_repl(repl);
        Ok(session_handle_map(KIND_BASH, id, None))
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, named_args, state);
        Err(unix_only_error())
    }
}

/// `pex.send_line(session, line) -> null`
pub fn send_line(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let handle = args.first().ok_or_else(|| {
            CorvoError::invalid_argument("pex.send_line requires a session handle")
        })?;
        let line = args
            .get(1)
            .and_then(|v| v.as_string())
            .ok_or_else(|| CorvoError::invalid_argument("pex.send_line requires a line string"))?;
        with_pex_session(state, handle, |entry| {
            with_pex_io(entry, |s| {
                s.send_line(line.as_str()).map_err(map_rexpect_err)?;
                Ok(Value::Null)
            })
        })
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, _named_args, state);
        Err(unix_only_error())
    }
}

/// `pex.send(session, text) -> null` — sends text and flushes to the process.
pub fn send(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let handle = args
            .first()
            .ok_or_else(|| CorvoError::invalid_argument("pex.send requires a session handle"))?;
        let text = args
            .get(1)
            .and_then(|v| v.as_string())
            .ok_or_else(|| CorvoError::invalid_argument("pex.send requires a text string"))?;
        with_pex_session(state, handle, |entry| {
            with_pex_io(entry, |s| {
                s.send(text.as_str()).map_err(map_rexpect_err)?;
                s.flush().map_err(map_rexpect_err)?;
                Ok(Value::Null)
            })
        })
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, _named_args, state);
        Err(unix_only_error())
    }
}

/// `pex.send_control(session, char) -> null` — e.g. `"c"` for Ctrl-C.
pub fn send_control(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let handle = args.first().ok_or_else(|| {
            CorvoError::invalid_argument("pex.send_control requires a session handle")
        })?;
        let ch_str = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
            CorvoError::invalid_argument("pex.send_control requires a single-character string")
        })?;
        let ch = ch_str.chars().next().ok_or_else(|| {
            CorvoError::invalid_argument("pex.send_control requires a non-empty character string")
        })?;
        with_pex_session(state, handle, |entry| {
            with_pex_io(entry, |s| {
                s.send_control(ch).map_err(map_rexpect_err)?;
                Ok(Value::Null)
            })
        })
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, _named_args, state);
        Err(unix_only_error())
    }
}

/// `pex.read_line(session) -> string`
pub fn read_line(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let handle = args.first().ok_or_else(|| {
            CorvoError::invalid_argument("pex.read_line requires a session handle")
        })?;
        with_pex_session(state, handle, |entry| {
            with_pex_io(entry, |s| {
                let line = s.read_line().map_err(map_rexpect_err)?;
                Ok(Value::String(line))
            })
        })
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, _named_args, state);
        Err(unix_only_error())
    }
}

/// `pex.exp_string(session, needle) -> map` with key `before`.
pub fn exp_string(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let handle = args.first().ok_or_else(|| {
            CorvoError::invalid_argument("pex.exp_string requires a session handle")
        })?;
        let needle = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
            CorvoError::invalid_argument("pex.exp_string requires a needle string")
        })?;
        with_pex_session(state, handle, |entry| {
            with_pex_io(entry, |s| {
                let before = s.exp_string(needle.as_str()).map_err(map_rexpect_err)?;
                let mut m = HashMap::new();
                m.insert("before".to_string(), Value::String(before));
                Ok(Value::Map(m))
            })
        })
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, _named_args, state);
        Err(unix_only_error())
    }
}

/// `pex.exp_regex(session, pattern) -> map` with keys `before` and `match`.
pub fn exp_regex(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let handle = args.first().ok_or_else(|| {
            CorvoError::invalid_argument("pex.exp_regex requires a session handle")
        })?;
        let pattern = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
            CorvoError::invalid_argument("pex.exp_regex requires a regex pattern string")
        })?;
        with_pex_session(state, handle, |entry| {
            with_pex_io(entry, |s| {
                let (before, matched) = s.exp_regex(pattern.as_str()).map_err(map_rexpect_err)?;
                let mut m = HashMap::new();
                m.insert("before".to_string(), Value::String(before));
                m.insert("match".to_string(), Value::String(matched));
                Ok(Value::Map(m))
            })
        })
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, _named_args, state);
        Err(unix_only_error())
    }
}

/// `pex.exp_eof(session) -> string` — remaining output when the child exits.
pub fn exp_eof(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let handle = args
            .first()
            .ok_or_else(|| CorvoError::invalid_argument("pex.exp_eof requires a session handle"))?;
        with_pex_session(state, handle, |entry| {
            with_pex_io(entry, |s| {
                let rest = s.exp_eof().map_err(map_rexpect_err)?;
                Ok(Value::String(rest))
            })
        })
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, _named_args, state);
        Err(unix_only_error())
    }
}

/// `pex.execute(session, cmd, ready_regex) -> null` — bash/repl sessions only.
pub fn execute(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let handle = args
            .first()
            .ok_or_else(|| CorvoError::invalid_argument("pex.execute requires a session handle"))?;
        let cmd = args
            .get(1)
            .and_then(|v| v.as_string())
            .ok_or_else(|| CorvoError::invalid_argument("pex.execute requires a command string"))?;
        let ready = args.get(2).and_then(|v| v.as_string()).ok_or_else(|| {
            CorvoError::invalid_argument("pex.execute requires a ready_regex string")
        })?;
        let id = session_id_from_value(handle)?;
        state.pex().with_entry_mut(id, |entry| match entry {
            PexEntry::Repl(r) => {
                r.execute(cmd.as_str(), ready.as_str())
                    .map_err(map_rexpect_err)?;
                Ok(Value::Null)
            }
            PexEntry::Session(_) => Err(CorvoError::invalid_argument(
                "pex.execute requires a pex.spawn_bash session",
            )),
        })
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, _named_args, state);
        Err(unix_only_error())
    }
}

/// `pex.wait_for_prompt(session) -> string` — bash/repl sessions only.
pub fn wait_for_prompt(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let handle = args.first().ok_or_else(|| {
            CorvoError::invalid_argument("pex.wait_for_prompt requires a session handle")
        })?;
        let id = session_id_from_value(handle)?;
        state.pex().with_entry_mut(id, |entry| match entry {
            PexEntry::Repl(r) => {
                let out = r.wait_for_prompt().map_err(map_rexpect_err)?;
                Ok(Value::String(out))
            }
            PexEntry::Session(_) => Err(CorvoError::invalid_argument(
                "pex.wait_for_prompt requires a pex.spawn_bash session",
            )),
        })
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, _named_args, state);
        Err(unix_only_error())
    }
}

/// `pex.close(session) -> null` — drop the session handle.
pub fn close(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    #[cfg(all(unix, feature = "stdlib-pex"))]
    {
        let handle = args
            .first()
            .ok_or_else(|| CorvoError::invalid_argument("pex.close requires a session handle"))?;
        let id = session_id_from_value(handle)?;
        state.pex().remove(id)?;
        Ok(Value::Null)
    }
    #[cfg(not(all(unix, feature = "stdlib-pex")))]
    {
        let _ = (args, _named_args, state);
        Err(unix_only_error())
    }
}

#[macro_export]
macro_rules! pex_spawn {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.spawn", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.spawn", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! pex_spawn_bash {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.spawn_bash", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.spawn_bash", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! pex_send_line {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.send_line", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.send_line", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! pex_send {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.send", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.send", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! pex_send_control {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.send_control", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.send_control", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! pex_read_line {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.read_line", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.read_line", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! pex_exp_string {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.exp_string", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.exp_string", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! pex_exp_regex {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.exp_regex", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.exp_regex", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! pex_exp_eof {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.exp_eof", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.exp_eof", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! pex_execute {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.execute", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.execute", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! pex_wait_for_prompt {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.wait_for_prompt", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.wait_for_prompt", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! pex_close {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.close", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("pex.close", &[$($arg),*], &$kwargs, $state)
    };
}

#[cfg(all(test, unix, feature = "stdlib-pex"))]
mod tests {
    use super::*;
    use crate::runtime::RuntimeState;

    fn empty_args() -> HashMap<String, Value> {
        HashMap::new()
    }

    #[test]
    fn spawn_cat_roundtrip() {
        let state = RuntimeState::new();
        let session = spawn(
            &[Value::String("cat".into()), Value::Number(5000.0)],
            &empty_args(),
            &state,
        )
        .unwrap();
        send_line(
            &[session.clone(), Value::String("hello, pex".into())],
            &empty_args(),
            &state,
        )
        .unwrap();
        let out = exp_string(
            &[session.clone(), Value::String("hello, pex".into())],
            &empty_args(),
            &state,
        )
        .unwrap();
        let m = out.as_map().unwrap();
        assert!(m.contains_key("before"));
        close(&[session], &empty_args(), &state).unwrap();
    }

    #[test]
    fn spawn_bash_echo_ok() {
        let state = RuntimeState::new();
        let session = spawn_bash(&[Value::Number(5000.0)], &empty_args(), &state).unwrap();
        send_line(
            &[session.clone(), Value::String("echo ok".into())],
            &empty_args(),
            &state,
        )
        .unwrap();
        exp_string(
            &[session.clone(), Value::String("ok".into())],
            &empty_args(),
            &state,
        )
        .unwrap();
        wait_for_prompt(&[session.clone()], &empty_args(), &state).unwrap();
        close(&[session], &empty_args(), &state).unwrap();
    }

    #[test]
    fn execute_requires_bash_session() {
        let state = RuntimeState::new();
        let session = spawn(
            &[Value::String("cat".into()), Value::Number(5000.0)],
            &empty_args(),
            &state,
        )
        .unwrap();
        let err = execute(
            &[
                session.clone(),
                Value::String("echo hi".into()),
                Value::String("hi".into()),
            ],
            &empty_args(),
            &state,
        )
        .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("spawn_bash"), "{msg}");
        close(&[session], &empty_args(), &state).unwrap();
    }

    #[test]
    fn close_invalidates_handle() {
        let state = RuntimeState::new();
        let session = spawn(
            &[Value::String("cat".into()), Value::Number(5000.0)],
            &empty_args(),
            &state,
        )
        .unwrap();
        close(&[session.clone()], &empty_args(), &state).unwrap();
        let err =
            send_line(&[session, Value::String("x".into())], &empty_args(), &state).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unknown or closed"), "{msg}");
    }
}
