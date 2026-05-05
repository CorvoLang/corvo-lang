use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn get(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let url = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("http.get requires a URL"))?;

    let headers = args.get(1).and_then(|v| v.as_map()).cloned();

    let client = reqwest::blocking::Client::new();
    let mut request = client.get(url);

    if let Some(h) = headers {
        for (key, value) in h {
            if let Some(v) = value.as_string() {
                request = request.header(key, v);
            }
        }
    }

    let response = request
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );
    result.insert("headers".to_string(), Value::Map(HashMap::new()));

    Ok(Value::Map(result))
}

pub fn post(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let url = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("http.post requires a URL"))?;

    let body = args
        .get(1)
        .and_then(|v| v.as_string())
        .cloned()
        .unwrap_or_default();

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url)
        .body(body)
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );

    Ok(Value::Map(result))
}

pub fn put(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let url = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("http.put requires a URL"))?;

    let body = args
        .get(1)
        .and_then(|v| v.as_string())
        .cloned()
        .unwrap_or_default();

    let client = reqwest::blocking::Client::new();
    let response = client
        .put(url)
        .body(body)
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );

    Ok(Value::Map(result))
}

pub fn delete(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let url = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("http.delete requires a URL"))?;

    let client = reqwest::blocking::Client::new();
    let response = client
        .delete(url)
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );

    Ok(Value::Map(result))
}

#[macro_export]
macro_rules! http_get {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("http.get", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("http.get", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! http_post {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("http.post", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("http.post", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! http_put {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("http.put", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("http.put", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! http_delete {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("http.delete", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("http.delete", &[$($arg),*], &$kwargs, $state)
    };
}
