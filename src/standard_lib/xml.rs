use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn parse_value(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let data = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("xml.parse requires a string"))?;

    let parsed = quick_xml::de::from_str::<serde_json::Value>(data)
        .map_err(|e| CorvoError::parsing(e.to_string()))?;

    crate::standard_lib::json::json_to_value(&parsed)
}

#[macro_export]
macro_rules! xml_parse {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("xml.parse", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("xml.parse", &[$($arg),*], &$kwargs, $state)
    };
}
