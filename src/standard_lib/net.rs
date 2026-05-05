use crate::runtime::{RuntimeState, TcpRegistry};
use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;
use std::net::TcpListener;

const KIND_LISTENER: &str = "tcp_listener";
const KIND_STREAM: &str = "tcp_stream";

fn listener_id_from_value(v: &Value) -> CorvoResult<u64> {
    let m = v.as_map().ok_or_else(|| {
        CorvoError::invalid_argument("expected tcp_listener handle map from net.tcp_listen")
    })?;
    let kind = m
        .get("kind")
        .and_then(|x| x.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| CorvoError::invalid_argument("tcp_listener handle missing string kind"))?;
    if kind != KIND_LISTENER {
        return Err(CorvoError::invalid_argument(
            "expected tcp_listener handle (wrong kind)",
        ));
    }
    let id = m
        .get("id")
        .and_then(|x| x.as_number())
        .ok_or_else(|| CorvoError::invalid_argument("tcp_listener handle missing numeric id"))?;
    if id < 0.0 || id.fract() != 0.0 {
        return Err(CorvoError::invalid_argument(
            "tcp_listener handle id must be a non-negative integer",
        ));
    }
    Ok(id as u64)
}

fn stream_id_from_value(v: &Value) -> CorvoResult<u64> {
    let m = v.as_map().ok_or_else(|| {
        CorvoError::invalid_argument(
            "expected tcp_stream handle map from net.tcp_connect / net.tcp_accept",
        )
    })?;
    let kind = m
        .get("kind")
        .and_then(|x| x.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| CorvoError::invalid_argument("tcp_stream handle missing string kind"))?;
    if kind != KIND_STREAM {
        return Err(CorvoError::invalid_argument(
            "expected tcp_stream handle (wrong kind)",
        ));
    }
    let id = m
        .get("id")
        .and_then(|x| x.as_number())
        .ok_or_else(|| CorvoError::invalid_argument("tcp_stream handle missing numeric id"))?;
    if id < 0.0 || id.fract() != 0.0 {
        return Err(CorvoError::invalid_argument(
            "tcp_stream handle id must be a non-negative integer",
        ));
    }
    Ok(id as u64)
}

fn listener_handle_map(id: u64, local_addr: String) -> Value {
    let mut m = HashMap::new();
    m.insert("kind".to_string(), Value::String(KIND_LISTENER.to_string()));
    m.insert("id".to_string(), Value::Number(id as f64));
    m.insert("local_addr".to_string(), Value::String(local_addr));
    Value::Map(m)
}

fn stream_handle_map(id: u64, local_addr: String, peer_addr: String) -> Value {
    let mut m = HashMap::new();
    m.insert("kind".to_string(), Value::String(KIND_STREAM.to_string()));
    m.insert("id".to_string(), Value::Number(id as f64));
    m.insert("local_addr".to_string(), Value::String(local_addr));
    m.insert("peer_addr".to_string(), Value::String(peer_addr));
    Value::Map(m)
}

/// `net.tcp_listen(address: string) -> map`  
/// Binds a TCP listener; `address` is e.g. `"127.0.0.1:8080"`.
pub fn tcp_listen(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    let addr = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("net.tcp_listen requires an address string"))?;
    let listener =
        TcpListener::bind(addr.as_str()).map_err(|e| CorvoError::network(e.to_string()))?;
    let (id, local_addr) = state.tcp().insert_listener(listener)?;
    Ok(listener_handle_map(id, local_addr))
}

/// `net.tcp_accept(listener: map) -> map`  
/// Blocks until a connection is accepted; returns a `tcp_stream` handle.
pub fn tcp_accept(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    let handle = args.first().ok_or_else(|| {
        CorvoError::invalid_argument("net.tcp_accept requires a tcp_listener handle")
    })?;
    let lid = listener_id_from_value(handle)?;
    let (sid, local_addr, peer_addr) = state.tcp().accept(lid)?;
    Ok(stream_handle_map(sid, local_addr, peer_addr))
}

/// `net.tcp_close_listener(listener: map) -> null`  
/// Stops accepting; the handle is invalid afterward.
pub fn tcp_close_listener(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    let handle = args.first().ok_or_else(|| {
        CorvoError::invalid_argument("net.tcp_close_listener requires a tcp_listener handle")
    })?;
    let lid = listener_id_from_value(handle)?;
    state.tcp().remove_listener(lid)?;
    Ok(Value::Null)
}

/// `net.tcp_connect(address: string) -> map`  
/// Connects to a remote peer; returns a `tcp_stream` handle.
pub fn tcp_connect(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    let addr = args.first().and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("net.tcp_connect requires an address string")
    })?;
    let stream = TcpRegistry::connect(addr.as_str())?;
    let (id, local_addr, peer_addr) = state.tcp().insert_stream(stream)?;
    Ok(stream_handle_map(id, local_addr, peer_addr))
}

/// `net.tcp_read(stream: map, max_bytes: number) -> string`  
/// Reads up to `max_bytes`; empty string means EOF or no data yet (TCP read returned 0).  
/// Non-UTF-8 bytes are replaced (lossy decode).
pub fn tcp_read(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    let handle = args
        .first()
        .ok_or_else(|| CorvoError::invalid_argument("net.tcp_read requires a tcp_stream handle"))?;
    let max = args
        .get(1)
        .and_then(|v| v.as_number())
        .ok_or_else(|| CorvoError::invalid_argument("net.tcp_read requires max_bytes number"))?;
    if max < 0.0 || max.fract() != 0.0 {
        return Err(CorvoError::invalid_argument(
            "net.tcp_read max_bytes must be a non-negative integer",
        ));
    }
    let sid = stream_id_from_value(handle)?;
    let s = state.tcp().read_stream(sid, max as usize)?;
    Ok(Value::String(s))
}

/// `net.tcp_write(stream: map, data: string) -> null`
pub fn tcp_write(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    let handle = args.first().ok_or_else(|| {
        CorvoError::invalid_argument("net.tcp_write requires a tcp_stream handle")
    })?;
    let data = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("net.tcp_write requires a string body"))?;
    let sid = stream_id_from_value(handle)?;
    state.tcp().write_stream(sid, data.as_bytes())?;
    Ok(Value::Null)
}

/// `net.tcp_close(stream: map) -> null`
pub fn tcp_close(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    let handle = args.first().ok_or_else(|| {
        CorvoError::invalid_argument("net.tcp_close requires a tcp_stream handle")
    })?;
    let sid = stream_id_from_value(handle)?;
    state.tcp().remove_stream(sid)?;
    Ok(Value::Null)
}

#[macro_export]
macro_rules! net_tcp_listen {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_listen", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_listen", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! net_tcp_accept {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_accept", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_accept", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! net_tcp_close_listener {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_close_listener", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_close_listener", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! net_tcp_connect {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_connect", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_connect", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! net_tcp_read {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_read", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_read", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! net_tcp_write {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_write", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_write", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! net_tcp_close {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_close", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("net.tcp_close", &[$($arg),*], &$kwargs, $state)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::RuntimeState;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn tcp_roundtrip_echo() {
        let rust_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = rust_listener.local_addr().unwrap().to_string();

        let server = thread::spawn(move || {
            let (mut s, _) = rust_listener.accept().unwrap();
            let mut buf = [0u8; 32];
            let n = s.read(&mut buf).unwrap();
            s.write_all(&buf[..n]).unwrap();
        });

        let state = RuntimeState::new();
        let c = tcp_connect(&[Value::String(addr)], &HashMap::new(), &state).unwrap();
        tcp_write(
            &[c.clone(), Value::String("ping".to_string())],
            &HashMap::new(),
            &state,
        )
        .unwrap();
        let out = tcp_read(&[c.clone(), Value::Number(32.0)], &HashMap::new(), &state).unwrap();
        assert_eq!(out, Value::String("ping".to_string()));
        tcp_close(&[c], &HashMap::new(), &state).unwrap();

        server.join().unwrap();
    }

    #[test]
    fn tcp_listen_accept_from_rust_client() {
        let state = RuntimeState::new();
        let ln = tcp_listen(
            &[Value::String("127.0.0.1:0".to_string())],
            &HashMap::new(),
            &state,
        )
        .unwrap();
        let addr = ln
            .as_map()
            .unwrap()
            .get("local_addr")
            .unwrap()
            .as_string()
            .unwrap()
            .clone();

        let client = thread::spawn(move || {
            let mut s = std::net::TcpStream::connect(addr).unwrap();
            s.write_all(b"yo").unwrap();
        });

        let stream = tcp_accept(std::slice::from_ref(&ln), &HashMap::new(), &state).unwrap();
        let data = tcp_read(
            &[stream.clone(), Value::Number(16.0)],
            &HashMap::new(),
            &state,
        )
        .unwrap();
        assert_eq!(data, Value::String("yo".to_string()));
        tcp_close(&[stream], &HashMap::new(), &state).unwrap();

        tcp_close_listener(&[ln], &HashMap::new(), &state).unwrap();
        client.join().unwrap();
    }
}
