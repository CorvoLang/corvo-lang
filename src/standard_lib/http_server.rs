use crate::type_system::Value;
use crate::{CorvoError, CorvoResult, RuntimeState};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

pub type NativeCallback =
    Arc<dyn Fn(&[Value], &mut RuntimeState) -> CorvoResult<Value> + Send + Sync>;

pub struct HttpServer;

impl HttpServer {
    /// Maximum HTTP request body size accepted when reading `Content-Length` (DoS mitigation).
    const MAX_HTTP_BODY_BYTES: usize = 10 * 1024 * 1024;

    pub fn exec_http_listen_native(
        port_val: Value,
        req_ident: &str,
        resp_ident: &str,
        shared_vars: &[String],
        proc: NativeCallback,
        state: &mut RuntimeState,
    ) -> CorvoResult<()> {
        let port = match port_val {
            Value::Number(n) if (0.0..=65535.0).contains(&n) && n.fract() == 0.0 => n as u16,
            _ => {
                return Err(CorvoError::r#type(
                    "http_listen port must be an integer between 0 and 65535",
                ))
            }
        };

        let listener = std::net::TcpListener::bind(format!("0.0.0.0:{}", port))
            .map_err(|e| CorvoError::runtime(format!("Failed to bind to port {}: {}", port, e)))?;

        let shared_arcs: Vec<Arc<Mutex<Value>>> = shared_vars
            .iter()
            .map(|name| {
                let val = state.var_get(name).unwrap_or(Value::Null);
                Arc::new(Mutex::new(val))
            })
            .collect::<Vec<_>>();

        for stream in listener.incoming() {
            let mut stream = stream
                .map_err(|e| CorvoError::runtime(format!("Failed to accept connection: {}", e)))?;

            let req_map = match Self::parse_http_request(&mut stream) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let arcs: Vec<Arc<Mutex<Value>>> = shared_arcs.iter().map(Arc::clone).collect();
            let thread_state = state.clone();
            let req_ident_clone = req_ident.to_string();
            let resp_ident_clone = resp_ident.to_string();
            let shared_vars_clone = shared_vars.to_vec();
            let proc_clone = Arc::clone(&proc);

            std::thread::spawn(move || {
                let mut scope_state = RuntimeState::new();
                let snapshots = Self::init_http_scope(
                    &req_ident_clone,
                    &resp_ident_clone,
                    &shared_vars_clone,
                    &arcs,
                    &thread_state,
                    req_map,
                    &mut scope_state,
                );

                let result = (proc_clone)(&[], &mut scope_state);
                let success = result.is_ok();

                let _ = Self::write_http_response(
                    &mut stream,
                    &scope_state,
                    &resp_ident_clone,
                    &shared_vars_clone,
                    &arcs,
                    &snapshots,
                    success,
                );
            });
        }
        Ok(())
    }

    fn extract_http_content_length(header_text: &str) -> usize {
        header_text
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.trim().eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    fn parse_http_request(stream: &mut TcpStream) -> CorvoResult<HashMap<String, Value>> {
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(30)))
            .map_err(|e| CorvoError::runtime(format!("Failed to set read timeout: {}", e)))?;

        let peer_ip = stream
            .peer_addr()
            .map(|a| a.ip().to_string())
            .unwrap_or_default();

        Self::parse_http_request_from_reader(stream, &peer_ip)
    }

    fn parse_http_request_from_reader<R: Read>(
        reader: &mut R,
        peer_ip: &str,
    ) -> CorvoResult<HashMap<String, Value>> {
        let mut request_raw = Vec::new();
        let mut buffer = [0; 1024];
        let mut header_end = 0;

        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .map_err(|e| CorvoError::runtime(format!("Failed to read request: {}", e)))?;
            if bytes_read == 0 {
                break;
            }
            request_raw.extend_from_slice(&buffer[..bytes_read]);

            if let Some(pos) = request_raw.windows(4).position(|w| w == b"\r\n\r\n") {
                header_end = pos + 4;
                break;
            }
            if let Some(pos) = request_raw.windows(2).position(|w| w == b"\n\n") {
                header_end = pos + 2;
                break;
            }

            if request_raw.len() > 1024 * 1024 {
                return Err(CorvoError::runtime("Headers too large"));
            }
        }

        let header_cow = String::from_utf8_lossy(&request_raw[..header_end]);
        let content_length = Self::extract_http_content_length(header_cow.as_ref());

        if content_length > Self::MAX_HTTP_BODY_BYTES {
            return Err(CorvoError::runtime(format!(
                "Request body too large (max {} bytes)",
                Self::MAX_HTTP_BODY_BYTES
            )));
        }

        let mut body_bytes = request_raw[header_end..].to_vec();
        if body_bytes.len() < content_length {
            let mut body_buffer = vec![0; content_length - body_bytes.len()];
            reader
                .read_exact(&mut body_buffer)
                .map_err(|e| CorvoError::runtime(format!("Failed to read body: {}", e)))?;
            body_bytes.extend_from_slice(&body_buffer);
        }

        let mut full_request = Vec::with_capacity(header_end + body_bytes.len());
        full_request.extend_from_slice(&request_raw[..header_end]);
        full_request.extend_from_slice(&body_bytes);

        Self::parse_http_raw(&full_request, header_end, peer_ip, header_cow.as_ref())
    }

    fn parse_http_raw(
        request_raw: &[u8],
        header_end: usize,
        peer_ip: &str,
        header_text: &str,
    ) -> CorvoResult<HashMap<String, Value>> {
        let mut req_map = HashMap::new();
        req_map.insert("ip".to_string(), Value::String(peer_ip.to_string()));

        let mut header_lines = header_text.lines();
        if let Some(first_line) = header_lines.next() {
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() >= 2 {
                req_map.insert("method".to_string(), Value::String(parts[0].to_string()));
                req_map.insert("path".to_string(), Value::String(parts[1].to_string()));
            }
        }

        let mut headers = HashMap::new();
        for line in header_lines {
            if let Some((k, v)) = line.split_once(':') {
                headers.insert(k.trim().to_lowercase(), Value::String(v.trim().to_string()));
            }
        }
        req_map.insert("headers".to_string(), Value::Map(headers));

        let body = String::from_utf8_lossy(&request_raw[header_end..]).to_string();
        req_map.insert("body".to_string(), Value::String(body));

        Ok(req_map)
    }

    fn init_http_scope(
        req_ident: &str,
        resp_ident: &str,
        shared_vars: &[String],
        shared_arcs: &[Arc<Mutex<Value>>],
        base_state: &RuntimeState,
        req_map: HashMap<String, Value>,
        scope_state: &mut RuntimeState,
    ) -> Vec<Value> {
        *scope_state = base_state.clone();
        scope_state.var_set(req_ident.to_string(), Value::Map(req_map));

        let mut initial_resp = HashMap::new();
        initial_resp.insert("status".to_string(), Value::Number(200.0));
        initial_resp.insert("body".to_string(), Value::String(String::new()));
        initial_resp.insert("headers".to_string(), Value::Map(HashMap::new()));
        scope_state.var_set(resp_ident.to_string(), Value::Map(initial_resp));

        let mut snapshots = Vec::with_capacity(shared_arcs.len());
        for (i, arc) in shared_arcs.iter().enumerate() {
            let val = arc.lock().unwrap().clone();
            snapshots.push(val.clone());
            scope_state.var_set(shared_vars[i].clone(), val);
        }
        snapshots
    }

    fn write_http_response(
        stream: &mut TcpStream,
        scope_state: &RuntimeState,
        resp_ident: &str,
        shared_vars: &[String],
        shared_arcs: &[Arc<Mutex<Value>>],
        snapshots: &[Value],
        success: bool,
    ) -> CorvoResult<()> {
        let resp_val = scope_state.var_get(resp_ident).unwrap_or(Value::Null);
        let (status, body, headers) = match resp_val {
            Value::Map(m) => {
                let s = m.get("status").and_then(|v| v.as_number()).unwrap_or(200.0) as u16;
                let b = m
                    .get("body")
                    .and_then(|v| v.as_string())
                    .cloned()
                    .unwrap_or_default();
                let mut h = HashMap::new();
                if let Some(Value::Map(hm)) = m.get("headers") {
                    for (k, v) in hm {
                        if let Some(vs) = v.as_string() {
                            h.insert(k.clone(), vs.clone());
                        }
                    }
                }
                (s, b, h)
            }
            _ => (
                if success { 200 } else { 500 },
                String::new(),
                HashMap::new(),
            ),
        };

        let mut response = format!("HTTP/1.1 {} OK\r\n", status);
        if !headers.contains_key("content-type") {
            response.push_str("Content-Type: text/plain\r\n");
        }
        for (k, v) in headers {
            response.push_str(&format!("{}: {}\r\n", k, v));
        }
        response.push_str(&format!("Content-Length: {}\r\n", body.len()));
        response.push_str("\r\n");
        response.push_str(&body);

        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();

        for (i, arc) in shared_arcs.iter().enumerate() {
            let thread_final = scope_state.var_get(&shared_vars[i]).unwrap_or(Value::Null);
            let mut guard = arc.lock().unwrap();
            let current = guard.clone();
            *guard = Value::merge_shared_writeback(&snapshots[i], &thread_final, &current);
        }
        Ok(())
    }
}
