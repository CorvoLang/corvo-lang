use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn resolve(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let hostname = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("dns.resolve requires a hostname"))?;

    let ips = dns_lookup::lookup_host(hostname)
        .map_err(|e| CorvoError::network(e.to_string()))?
        .into_iter()
        .map(|ip| Value::String(ip.to_string()))
        .collect();

    Ok(Value::List(ips))
}

pub fn lookup(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let ip = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("dns.lookup requires an IP address"))?;

    let hostname = dns_lookup::lookup_addr(
        &ip.parse()
            .map_err(|_| CorvoError::invalid_argument("Invalid IP address"))?,
    )
    .map_err(|e| CorvoError::network(e.to_string()))?;

    Ok(Value::String(hostname))
}

#[macro_export]
macro_rules! dns_resolve {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("dns.resolve", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("dns.resolve", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! dns_lookup {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("dns.lookup", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("dns.lookup", &[$($arg),*], &$kwargs, $state)
    };
}
