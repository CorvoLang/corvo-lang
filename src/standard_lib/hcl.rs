use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn parse_value(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let data = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("hcl.parse requires a string"))?;

    Ok(Value::String(data.clone()))
}

pub fn stringify(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let value = args
        .first()
        .ok_or_else(|| CorvoError::invalid_argument("hcl.stringify requires a value"))?;

    Ok(Value::String(value.to_string()))
}

#[macro_export]
macro_rules! hcl_parse {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("hcl.parse", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("hcl.parse", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! hcl_stringify {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("hcl.stringify", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("hcl.stringify", &[$($arg),*], &$kwargs, $state)
    };
}
