use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use crate::type_system::Type;

/// A procedure definition captured at the point of `@proc = procedure(...) { ... }`.
/// Procedures are not serialisable (they hold AST nodes), so manual impls are used
/// to make `Value` serde-compatible while preventing procedures from being stored as statics.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcedureValue {
    pub params: Vec<String>,
    pub body: Vec<crate::ast::stmt::Stmt>,
}

impl Serialize for ProcedureValue {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "procedures cannot be serialized as statics",
        ))
    }
}

impl<'de> Deserialize<'de> for ProcedureValue {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "procedures cannot be deserialized",
        ))
    }
}

/// A mutex-protected value shared across threads during `async_browse` execution.
///
/// The `Arc<Mutex<Value>>` is cloned (cheaply) into each spawned thread.  Each
/// thread briefly locks the mutex to take a snapshot of the current value before
/// running its procedure body, and locks it again to write the updated value back
/// when the body finishes.
#[derive(Debug, Clone)]
pub struct SharedValue(pub Arc<Mutex<Value>>);

impl PartialEq for SharedValue {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Serialize for SharedValue {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "shared values cannot be serialized as statics",
        ))
    }
}

impl<'de> Deserialize<'de> for SharedValue {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "shared values cannot be deserialized",
        ))
    }
}

pub type NativeCallback = Arc<
    dyn Fn(&[Value], &mut crate::runtime::RuntimeState) -> crate::CorvoResult<Value> + Send + Sync,
>;

#[derive(Clone, Default)]
pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Regex(String, String), // pattern, flags
    #[default]
    Null,
    Procedure(Box<ProcedureValue>),
    /// A native Rust procedure used in transpiled code.
    NativeProcedure {
        params: Vec<String>,
        callback: NativeCallback,
    },
    /// A mutex-protected value created during `async_browse` to allow threads to
    /// share a single accumulator variable safely.  This variant is internal-only
    /// and is never produced by ordinary Corvo code.
    Shared(Box<SharedValue>),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(v) => f.debug_tuple("String").field(v).finish(),
            Self::Number(v) => f.debug_tuple("Number").field(v).finish(),
            Self::Boolean(v) => f.debug_tuple("Boolean").field(v).finish(),
            Self::List(v) => f.debug_tuple("List").field(v).finish(),
            Self::Map(v) => f.debug_tuple("Map").field(v).finish(),
            Self::Regex(p, fl) => f.debug_tuple("Regex").field(p).field(fl).finish(),
            Self::Null => f.write_str("Null"),
            Self::Procedure(p) => f.debug_tuple("Procedure").field(p).finish(),
            Self::NativeProcedure { .. } => f.write_str("NativeProcedure(<native closure>)"),
            Self::Shared(s) => f.debug_tuple("Shared").field(s).finish(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Map(a), Self::Map(b)) => a == b,
            (Self::Regex(ap, af), Self::Regex(bp, bf)) => ap == bp && af == bf,
            (Self::Null, Self::Null) => true,
            (Self::Procedure(a), Self::Procedure(b)) => a == b,
            (
                Self::NativeProcedure {
                    params: ap,
                    callback: ac,
                },
                Self::NativeProcedure {
                    params: bp,
                    callback: bc,
                },
            ) => ap == bp && Arc::ptr_eq(ac, bc),
            (Self::Shared(a), Self::Shared(b)) => a == b,
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::String(a), Self::String(b)) => a.partial_cmp(b),
            (Self::Number(a), Self::Number(b)) => a.partial_cmp(b),
            (Self::Boolean(a), Self::Boolean(b)) => a.partial_cmp(b),
            (Self::List(a), Self::List(b)) => a.partial_cmp(b),
            (Self::Null, Self::Null) => Some(std::cmp::Ordering::Equal),
            (a, b) => a.to_string().partial_cmp(&b.to_string()),
        }
    }
}

impl Serialize for Value {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::String(s) => serializer.serialize_newtype_variant("Value", 0, "String", s),
            Self::Number(n) => serializer.serialize_newtype_variant("Value", 1, "Number", n),
            Self::Boolean(b) => serializer.serialize_newtype_variant("Value", 2, "Boolean", b),
            Self::List(l) => serializer.serialize_newtype_variant("Value", 3, "List", l),
            Self::Map(m) => serializer.serialize_newtype_variant("Value", 4, "Map", m),
            Self::Regex(p, f) => serializer
                .serialize_tuple_variant("Value", 5, "Regex", 2)
                .and_then(|mut tv| {
                    use serde::ser::SerializeTupleVariant;
                    tv.serialize_field(p)?;
                    tv.serialize_field(f)?;
                    tv.end()
                }),
            Self::Null => serializer.serialize_unit_variant("Value", 6, "Null"),
            Self::Procedure(_) => Err(serde::ser::Error::custom("procedures cannot be serialized")),
            Self::NativeProcedure { .. } => Err(serde::ser::Error::custom(
                "native procedures cannot be serialized",
            )),
            Self::Shared(_) => Err(serde::ser::Error::custom(
                "shared values cannot be serialized",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(variant_identifier)]
        enum Field {
            String,
            Number,
            Boolean,
            List,
            Map,
            Regex,
            Null,
        }

        struct ValueVisitor;
        impl<'de> serde::de::Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("enum Value")
            }

            fn visit_enum<A: serde::de::EnumAccess<'de>>(
                self,
                data: A,
            ) -> Result<Self::Value, A::Error> {
                use serde::de::VariantAccess;
                let (field, variant) = data.variant::<Field>()?;
                match field {
                    Field::String => Ok(Value::String(variant.newtype_variant()?)),
                    Field::Number => Ok(Value::Number(variant.newtype_variant()?)),
                    Field::Boolean => Ok(Value::Boolean(variant.newtype_variant()?)),
                    Field::List => Ok(Value::List(variant.newtype_variant()?)),
                    Field::Map => Ok(Value::Map(variant.newtype_variant()?)),
                    Field::Regex => {
                        struct RegexVisitor;
                        impl<'de> serde::de::Visitor<'de> for RegexVisitor {
                            type Value = (String, String);
                            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                                f.write_str("tuple (String, String)")
                            }
                            fn visit_seq<V: serde::de::SeqAccess<'de>>(
                                self,
                                mut seq: V,
                            ) -> Result<Self::Value, V::Error> {
                                let p = seq
                                    .next_element()?
                                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                                let f = seq
                                    .next_element()?
                                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                                Ok((p, f))
                            }
                        }
                        let (p, f) = variant.tuple_variant(2, RegexVisitor)?;
                        Ok(Value::Regex(p, f))
                    }
                    Field::Null => {
                        variant.unit_variant()?;
                        Ok(Value::Null)
                    }
                }
            }
        }
        deserializer.deserialize_enum(
            "Value",
            &[
                "String", "Number", "Boolean", "List", "Map", "Regex", "Null",
            ],
            ValueVisitor,
        )
    }
}

impl Value {
    pub fn r#type(&self) -> Type {
        match self {
            Self::String(_) => Type::String,
            Self::Number(_) => Type::Number,
            Self::Boolean(_) => Type::Boolean,
            Self::List(_) => Type::List,
            Self::Map(_) => Type::Map,
            Self::Regex(_, _) => Type::Regex,
            Self::Null => Type::Null,
            Self::Procedure(_) | Self::NativeProcedure { .. } => Type::Procedure,
            Self::Shared(sv) => sv.0.lock().unwrap().r#type(),
        }
    }

    pub fn as_string(&self) -> Option<&String> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<Value>> {
        match self {
            Self::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Self::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_regex(&self) -> Option<(&String, &String)> {
        match self {
            Self::Regex(pattern, flags) => Some((pattern, flags)),
            _ => None,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Boolean(b) => *b,
            Self::Null => false,
            Self::String(s) => !s.is_empty(),
            Self::Number(n) => *n != 0.0,
            Self::List(l) => !l.is_empty(),
            Self::Map(m) => !m.is_empty(),
            Self::Regex(pattern, _) => !pattern.is_empty(),
            Self::Procedure(_) | Self::NativeProcedure { .. } => true,
            Self::Shared(sv) => sv.0.lock().unwrap().is_truthy(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Self::Boolean(b) => write!(f, "{}", b),
            Self::List(l) => {
                let items: Vec<String> = l.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", items.join(", "))
            }
            Self::Map(m) => {
                let items: Vec<String> =
                    m.iter().map(|(k, v)| format!("\"{}\": {}", k, v)).collect();
                write!(f, "{{{}}}", items.join(", "))
            }
            Self::Regex(pattern, flags) => write!(f, "/{}/{}", pattern, flags),
            Self::Null => write!(f, "null"),
            Self::Procedure(_) | Self::NativeProcedure { .. } => write!(f, "<procedure>"),
            Self::Shared(sv) => write!(f, "{}", sv.0.lock().unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type() {
        assert_eq!(Value::String("test".to_string()).r#type(), Type::String);
        assert_eq!(Value::Number(42.0).r#type(), Type::Number);
        assert_eq!(Value::Boolean(true).r#type(), Type::Boolean);
        assert_eq!(Value::List(vec![]).r#type(), Type::List);
        assert_eq!(Value::Map(HashMap::new()).r#type(), Type::Map);
        assert_eq!(Value::Null.r#type(), Type::Null);
    }

    #[test]
    fn test_is_truthy() {
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Boolean(false).is_truthy());
        assert!(Value::Boolean(true).is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::String(String::new()).is_truthy());
        assert!(Value::Number(1.0).is_truthy());
        assert!(!Value::Number(0.0).is_truthy());
        assert!(Value::Number(-1.0).is_truthy());
        assert!(Value::List(vec![Value::Number(1.0)]).is_truthy());
        assert!(!Value::List(vec![]).is_truthy());
        let mut map = HashMap::new();
        map.insert("a".to_string(), Value::Number(1.0));
        assert!(Value::Map(map).is_truthy());
        assert!(!Value::Map(HashMap::new()).is_truthy());
    }

    #[test]
    fn test_display() {
        assert_eq!(Value::Number(42.0).to_string(), "42");
        assert_eq!(Value::Number(42.5).to_string(), "42.5");
        assert_eq!(Value::Boolean(true).to_string(), "true");
        assert_eq!(Value::Boolean(false).to_string(), "false");
        assert_eq!(Value::Null.to_string(), "null");
        assert_eq!(Value::String("hello".to_string()).to_string(), "hello");
    }

    #[test]
    fn test_display_list() {
        let list = Value::List(vec![Value::Number(1.0), Value::Number(2.0)]);
        let display = list.to_string();
        assert_eq!(display, "[1, 2]");
    }

    #[test]
    fn test_display_empty_list() {
        let list = Value::List(vec![]);
        assert_eq!(list.to_string(), "[]");
    }

    #[test]
    fn test_display_map() {
        let mut map = HashMap::new();
        map.insert("a".to_string(), Value::Number(1.0));
        let display = Value::Map(map).to_string();
        assert!(display.contains("\"a\""));
        assert!(display.contains("1"));
    }

    #[test]
    fn test_display_nested() {
        let inner = Value::List(vec![Value::Number(1.0), Value::Number(2.0)]);
        let outer = Value::List(vec![inner, Value::String("hello".to_string())]);
        let display = outer.to_string();
        assert!(display.contains("[1, 2]"));
        assert!(display.contains("hello"));
    }

    #[test]
    fn test_as_string() {
        assert_eq!(
            Value::String("hello".to_string()).as_string(),
            Some(&"hello".to_string())
        );
        assert_eq!(Value::Number(42.0).as_string(), None);
    }

    #[test]
    fn test_as_number() {
        assert_eq!(Value::Number(42.0).as_number(), Some(42.0));
        assert_eq!(Value::String("42".to_string()).as_number(), None);
    }

    #[test]
    fn test_as_bool() {
        assert_eq!(Value::Boolean(true).as_bool(), Some(true));
        assert_eq!(Value::Boolean(false).as_bool(), Some(false));
        assert_eq!(Value::Null.as_bool(), None);
    }

    #[test]
    fn test_as_list() {
        let items = vec![Value::Number(1.0)];
        assert!(Value::List(items.clone()).as_list().is_some());
        assert!(Value::Null.as_list().is_none());
    }

    #[test]
    fn test_as_map() {
        let map = HashMap::new();
        assert!(Value::Map(map).as_map().is_some());
        assert!(Value::Null.as_map().is_none());
    }

    #[test]
    fn test_default_value() {
        let val: Value = Default::default();
        assert_eq!(val, Value::Null);
    }

    #[test]
    fn test_clone_equality() {
        let original = Value::List(vec![Value::String("a".to_string()), Value::Number(1.0)]);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_number_display_integer_vs_float() {
        assert_eq!(Value::Number(0.0).to_string(), "0");
        assert_eq!(Value::Number(1.0).to_string(), "1");
        assert_eq!(Value::Number(-1.0).to_string(), "-1");
        assert_eq!(Value::Number(0.5).to_string(), "0.5");
        assert_eq!(Value::Number(100.25).to_string(), "100.25");
    }
}
