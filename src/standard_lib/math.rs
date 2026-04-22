use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn add(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() < 2 {
        return Err(CorvoError::invalid_argument("add requires 2 arguments"));
    }

    let a = args[0]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("add requires numbers"))?;
    let b = args[1]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("add requires numbers"))?;

    Ok(Value::Number(a + b))
}

pub fn sub(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() < 2 {
        return Err(CorvoError::invalid_argument("sub requires 2 arguments"));
    }

    let a = args[0]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("sub requires numbers"))?;
    let b = args[1]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("sub requires numbers"))?;

    Ok(Value::Number(a - b))
}

pub fn mul(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() < 2 {
        return Err(CorvoError::invalid_argument("mul requires 2 arguments"));
    }

    let a = args[0]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("mul requires numbers"))?;
    let b = args[1]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("mul requires numbers"))?;

    Ok(Value::Number(a * b))
}

pub fn div(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() < 2 {
        return Err(CorvoError::invalid_argument("div requires 2 arguments"));
    }

    let a = args[0]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("div requires numbers"))?;
    let b = args[1]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("div requires numbers"))?;

    if b == 0.0 {
        return Err(CorvoError::division_by_zero());
    }

    Ok(Value::Number(a / b))
}

pub fn modulo(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.len() < 2 {
        return Err(CorvoError::invalid_argument("mod requires 2 arguments"));
    }

    let a = args[0]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("mod requires numbers"))?;
    let b = args[1]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("mod requires numbers"))?;

    if b == 0.0 {
        return Err(CorvoError::division_by_zero());
    }

    Ok(Value::Number(a % b))
}

/// Maximum of two or more numbers.
pub fn max(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "math.max requires at least one number",
        ));
    }
    let mut m = args[0]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("math.max requires numbers"))?;
    for a in args.iter().skip(1) {
        let n = a
            .as_number()
            .ok_or_else(|| CorvoError::r#type("math.max requires numbers"))?;
        m = m.max(n);
    }
    Ok(Value::Number(m))
}

/// Minimum of two or more numbers.
pub fn min(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "math.min requires at least one number",
        ));
    }
    let mut m = args[0]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("math.min requires numbers"))?;
    for a in args.iter().skip(1) {
        let n = a
            .as_number()
            .ok_or_else(|| CorvoError::r#type("math.min requires numbers"))?;
        m = m.min(n);
    }
    Ok(Value::Number(m))
}

pub fn floor(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let n = args
        .first()
        .and_then(|v| v.as_number())
        .ok_or_else(|| CorvoError::invalid_argument("math.floor requires a number"))?;
    Ok(Value::Number(n.floor()))
}

pub fn ceil(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let n = args
        .first()
        .and_then(|v| v.as_number())
        .ok_or_else(|| CorvoError::invalid_argument("math.ceil requires a number"))?;
    Ok(Value::Number(n.ceil()))
}

pub fn round(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let n = args
        .first()
        .and_then(|v| v.as_number())
        .ok_or_else(|| CorvoError::invalid_argument("math.round requires a number"))?;
    Ok(Value::Number(n.round()))
}

pub fn random(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    if args.is_empty() {
        Ok(Value::Number(rng.gen::<f64>()))
    } else if args.len() == 1 {
        let limit = args[0]
            .as_number()
            .ok_or_else(|| CorvoError::r#type("math.random(limit) requires a number"))?;
        Ok(Value::Number(rng.gen_range(0.0..limit)))
    } else {
        let start = args[0]
            .as_number()
            .ok_or_else(|| CorvoError::r#type("math.random(start, end) requires numbers"))?;
        let end = args[1]
            .as_number()
            .ok_or_else(|| CorvoError::r#type("math.random(start, end) requires numbers"))?;
        Ok(Value::Number(rng.gen_range(start..end)))
    }
}

/// Format a byte size like GNU `ls --human-readable` (`si`: powers of 1000 instead of 1024).
pub fn human_bytes(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let n = args.first().and_then(|v| v.as_number()).ok_or_else(|| {
        CorvoError::invalid_argument("math.human_bytes requires a byte size (number)")
    })?;
    let si = args.get(1).and_then(|v| v.as_bool()).unwrap_or(false);
    let n = n.max(0.0);
    let base = if si { 1000.0 } else { 1024.0 };
    if n < base {
        return Ok(Value::String(format!("{:.0}", n)));
    }
    let suf = if si {
        ["B", "k", "M", "G", "T", "P", "E", "Z", "Y"]
    } else {
        ["B", "K", "M", "G", "T", "P", "E", "Z", "Y"]
    };
    let mut val = n;
    let mut idx = 0usize;
    while val >= base && idx + 1 < suf.len() {
        val /= base;
        idx += 1;
    }
    let out = if idx > 0 && val < 10.0 {
        format!("{:.1}{}", val, suf[idx])
    } else {
        format!("{:.0}{}", val.round(), suf[idx])
    };
    Ok(Value::String(out))
}
/// Parse a size string like "10K", "5M", "2G" into a number of bytes.
/// Supports both SI (K=1000) and binary (K=1024) suffixes if specified.
/// By default uses binary (1024).
pub fn parse_size(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let s = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("math.parse_size requires a string"))?;

    let s = s.trim();
    if s.is_empty() {
        return Ok(Value::Number(0.0));
    }

    let mut num_str = s.to_string();
    let mut multiplier: f64 = 1.0;

    if let Some(last_char) = s.chars().last() {
        match last_char.to_ascii_lowercase() {
            'k' => {
                multiplier = 1024.0;
                num_str.pop();
            }
            'm' => {
                multiplier = 1024.0 * 1024.0;
                num_str.pop();
            }
            'g' => {
                multiplier = 1024.0 * 1024.0 * 1024.0;
                num_str.pop();
            }
            't' => {
                multiplier = 1024.0 * 1024.0 * 1024.0 * 1024.0;
                num_str.pop();
            }
            'p' => {
                multiplier = 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0;
                num_str.pop();
            }
            'e' => {
                multiplier = 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0;
                num_str.pop();
            }
            '0'..='9' => {}
            _ => {
                // Check for SI suffixes like KB, MB
                if s.len() >= 2 {
                    let suffix = &s[s.len() - 2..].to_ascii_lowercase();
                    match suffix.as_str() {
                        "kb" => {
                            multiplier = 1000.0;
                            num_str.truncate(s.len() - 2);
                        }
                        "mb" => {
                            multiplier = 1000.0 * 1000.0;
                            num_str.truncate(s.len() - 2);
                        }
                        "gb" => {
                            multiplier = 1000.0 * 1000.0 * 1000.0;
                            num_str.truncate(s.len() - 2);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    let val = num_str
        .parse::<f64>()
        .map_err(|e| CorvoError::invalid_argument(format!("Invalid size format '{}': {}", s, e)))?;

    Ok(Value::Number(val * multiplier))
}

/// Generate a list of numbers from start (inclusive) to end (exclusive).
/// Args: `end` (range [0, end)), or `start`, `end`, or `start`, `end`, `step`.
pub fn range(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let (start, end, step) = match args.len() {
        1 => (
            0.0,
            args[0]
                .as_number()
                .ok_or_else(|| CorvoError::r#type("math.range requires numbers"))?,
            1.0,
        ),
        2 => (
            args[0]
                .as_number()
                .ok_or_else(|| CorvoError::r#type("math.range requires numbers"))?,
            args[1]
                .as_number()
                .ok_or_else(|| CorvoError::r#type("math.range requires numbers"))?,
            1.0,
        ),
        3 => (
            args[0]
                .as_number()
                .ok_or_else(|| CorvoError::r#type("math.range requires numbers"))?,
            args[1]
                .as_number()
                .ok_or_else(|| CorvoError::r#type("math.range requires numbers"))?,
            args[2]
                .as_number()
                .ok_or_else(|| CorvoError::r#type("math.range requires numbers"))?,
        ),
        _ => {
            return Err(CorvoError::invalid_argument(
                "math.range requires 1, 2, or 3 arguments",
            ))
        }
    };

    if step == 0.0 {
        return Err(CorvoError::invalid_argument(
            "math.range step cannot be zero",
        ));
    }

    let mut result = Vec::new();
    let mut current = start;

    if step > 0.0 {
        while current < end {
            result.push(Value::Number(current));
            current += step;
        }
    } else {
        while current > end {
            result.push(Value::Number(current));
            current += step;
        }
    }

    Ok(Value::List(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_args() -> HashMap<String, Value> {
        HashMap::new()
    }

    #[test]
    fn test_max() {
        let args = vec![Value::Number(2.0), Value::Number(5.0), Value::Number(3.0)];
        assert_eq!(max(&args, &empty_args()).unwrap(), Value::Number(5.0));
    }

    #[test]
    fn test_add() {
        let args = vec![Value::Number(2.0), Value::Number(3.0)];
        assert_eq!(add(&args, &empty_args()).unwrap(), Value::Number(5.0));
    }

    #[test]
    fn test_add_negative() {
        let args = vec![Value::Number(-1.0), Value::Number(3.0)];
        assert_eq!(add(&args, &empty_args()).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn test_sub() {
        let args = vec![Value::Number(10.0), Value::Number(3.0)];
        assert_eq!(sub(&args, &empty_args()).unwrap(), Value::Number(7.0));
    }

    #[test]
    fn test_mul() {
        let args = vec![Value::Number(4.0), Value::Number(5.0)];
        assert_eq!(mul(&args, &empty_args()).unwrap(), Value::Number(20.0));
    }

    #[test]
    fn test_div() {
        let args = vec![Value::Number(10.0), Value::Number(2.0)];
        assert_eq!(div(&args, &empty_args()).unwrap(), Value::Number(5.0));
    }

    #[test]
    fn test_div_by_zero() {
        let args = vec![Value::Number(10.0), Value::Number(0.0)];
        assert!(div(&args, &empty_args()).is_err());
    }

    #[test]
    fn test_modulo() {
        let args = vec![Value::Number(10.0), Value::Number(3.0)];
        assert_eq!(modulo(&args, &empty_args()).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_mod_by_zero() {
        let args = vec![Value::Number(10.0), Value::Number(0.0)];
        assert!(modulo(&args, &empty_args()).is_err());
    }

    #[test]
    fn test_add_wrong_type() {
        let args = vec![Value::String("a".to_string()), Value::Number(1.0)];
        assert!(add(&args, &empty_args()).is_err());
    }

    #[test]
    fn test_add_too_few_args() {
        let args = vec![Value::Number(1.0)];
        assert!(add(&args, &empty_args()).is_err());
    }

    #[test]
    fn test_add_zero() {
        let args = vec![Value::Number(5.0), Value::Number(0.0)];
        assert_eq!(add(&args, &empty_args()).unwrap(), Value::Number(5.0));
    }

    #[test]
    fn test_mul_by_zero() {
        let args = vec![Value::Number(5.0), Value::Number(0.0)];
        assert_eq!(mul(&args, &empty_args()).unwrap(), Value::Number(0.0));
    }

    #[test]
    fn test_div_float() {
        let args = vec![Value::Number(7.0), Value::Number(2.0)];
        assert_eq!(div(&args, &empty_args()).unwrap(), Value::Number(3.5));
    }

    #[test]
    fn human_k() {
        let args = vec![Value::Number(2048.0), Value::Boolean(false)];
        let s = human_bytes(&args, &empty_args()).unwrap();
        assert_eq!(s, Value::String("2.0K".to_string()));
    }
}
