use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn model(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let name = args
        .first()
        .and_then(|v| v.as_string())
        .cloned()
        .unwrap_or_default();

    let provider = args
        .get(1)
        .and_then(|v| v.as_string())
        .cloned()
        .unwrap_or_default();

    Ok(Value::String(format!("{}:{}", provider, name)))
}

pub fn prompt(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let _model = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("llm.prompt requires a model"))?;

    let prompt_text = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("llm.prompt requires a prompt"))?;

    Ok(Value::String(format!("[LLM Response to: {}]", prompt_text)))
}

pub fn embed(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let _model = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("llm.embed requires a model"))?;

    let _text = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("llm.embed requires text"))?;

    Ok(Value::List(vec![Value::Number(0.0); 768]))
}

pub fn chat(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let _id = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("llm.chat requires an id"))?;

    let _model = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("llm.chat requires a model"))?;

    let _messages = args
        .get(2)
        .and_then(|v| v.as_list())
        .ok_or_else(|| CorvoError::invalid_argument("llm.chat requires messages"))?;

    let mut response = HashMap::new();
    response.insert("role".to_string(), Value::String("assistant".to_string()));
    response.insert(
        "content".to_string(),
        Value::String("[LLM Chat Response]".to_string()),
    );

    Ok(Value::Map(response))
}

#[macro_export]
macro_rules! llm_model {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("llm.model", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("llm.model", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! llm_prompt {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("llm.prompt", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("llm.prompt", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! llm_embed {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("llm.embed", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("llm.embed", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! llm_chat {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("llm.chat", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("llm.chat", &[$($arg),*], &$kwargs, $state)
    };
}
