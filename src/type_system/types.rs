use super::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    String,
    Number,
    Boolean,
    List,
    Map,
    Regex,
    Null,
    Procedure,
    DatabasePool,
}

impl Type {
    pub fn from_value(value: &Value) -> Self {
        value.r#type()
    }

    pub fn parse_name(s: &str) -> Option<Self> {
        match s {
            "string" => Some(Self::String),
            "number" => Some(Self::Number),
            "boolean" => Some(Self::Boolean),
            "list" => Some(Self::List),
            "map" => Some(Self::Map),
            "regex" => Some(Self::Regex),
            "null" => Some(Self::Null),
            "procedure" => Some(Self::Procedure),
            "database_pool" => Some(Self::DatabasePool),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::List => "list",
            Self::Map => "map",
            Self::Regex => "regex",
            Self::Null => "null",
            Self::Procedure => "procedure",
            Self::DatabasePool => "database_pool",
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_name_valid() {
        assert_eq!(Type::parse_name("string"), Some(Type::String));
        assert_eq!(Type::parse_name("number"), Some(Type::Number));
        assert_eq!(Type::parse_name("boolean"), Some(Type::Boolean));
        assert_eq!(Type::parse_name("list"), Some(Type::List));
        assert_eq!(Type::parse_name("map"), Some(Type::Map));
        assert_eq!(Type::parse_name("null"), Some(Type::Null));
    }

    #[test]
    fn test_parse_name_invalid() {
        assert_eq!(Type::parse_name("unknown"), None);
        assert_eq!(Type::parse_name(""), None);
        assert_eq!(Type::parse_name("String"), None); // case sensitive
    }

    #[test]
    fn test_as_str() {
        assert_eq!(Type::String.as_str(), "string");
        assert_eq!(Type::Number.as_str(), "number");
        assert_eq!(Type::Boolean.as_str(), "boolean");
        assert_eq!(Type::List.as_str(), "list");
        assert_eq!(Type::Map.as_str(), "map");
        assert_eq!(Type::Null.as_str(), "null");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Type::String), "string");
        assert_eq!(format!("{}", Type::Number), "number");
        assert_eq!(format!("{}", Type::Boolean), "boolean");
    }

    #[test]
    fn test_from_value() {
        assert_eq!(
            Type::from_value(&crate::type_system::Value::String("".to_string())),
            Type::String
        );
        assert_eq!(
            Type::from_value(&crate::type_system::Value::Number(0.0)),
            Type::Number
        );
        assert_eq!(
            Type::from_value(&crate::type_system::Value::Boolean(false)),
            Type::Boolean
        );
        assert_eq!(
            Type::from_value(&crate::type_system::Value::Null),
            Type::Null
        );
    }

    #[test]
    fn test_equality() {
        assert_eq!(Type::String, Type::String);
        assert_ne!(Type::String, Type::Number);
    }
}
