use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

/// Build a `regex::Regex` from a Corvo regex value's pattern and flags.
///
/// The following flags are supported:
/// * `i` – case-insensitive matching (`(?i)`)
/// * `m` – multi-line mode (`(?m)`)
/// * `s` – dot-matches-newline (`(?s)`)
/// * `g` – global (no-op here; use `re.find_all` instead of `re.find`)
/// * `u` – Unicode (already the default in Rust's regex crate; ignored)
pub fn build_regex(pattern: &str, flags: &str) -> CorvoResult<regex::Regex> {
    let mut prefix = String::new();
    for ch in flags.chars() {
        match ch {
            'i' => prefix.push_str("(?i)"),
            'm' => prefix.push_str("(?m)"),
            's' => prefix.push_str("(?s)"),
            'g' | 'u' => {} // g = global (use find_all), u = unicode (default)
            _ => {}         // ignore unknown flags
        }
    }
    let full_pattern = format!("{}{}", prefix, pattern);
    regex::Regex::new(&full_pattern)
        .map_err(|e| CorvoError::runtime(format!("Invalid regex pattern: {}", e)))
}

/// Extract (pattern, flags) from the first argument, which must be a regex value.
fn extract_regex(v: Option<&Value>) -> CorvoResult<(&str, &str)> {
    v.and_then(|val| val.as_regex())
        .map(|(p, f)| (p.as_str(), f.as_str()))
        .ok_or_else(|| CorvoError::r#type("re method requires a regex as the first argument"))
}

/// `re.match(regex, string)` – returns `true` if the string contains a match.
pub fn is_match(args: &[Value], _named: &HashMap<String, Value>) -> CorvoResult<Value> {
    let (pattern, flags) = extract_regex(args.first())?;
    let text = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| CorvoError::r#type("re.match requires a string as the second argument"))?;
    let re = build_regex(pattern, flags)?;
    Ok(Value::Boolean(re.is_match(text)))
}

/// `re.find(regex, string)` – returns the first match as a string, or null.
pub fn find(args: &[Value], _named: &HashMap<String, Value>) -> CorvoResult<Value> {
    let (pattern, flags) = extract_regex(args.first())?;
    let text = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| CorvoError::r#type("re.find requires a string as the second argument"))?;
    let re = build_regex(pattern, flags)?;
    Ok(re
        .find(text)
        .map(|m| Value::String(m.as_str().to_string()))
        .unwrap_or(Value::Null))
}

/// `re.find_all(regex, string)` – returns all non-overlapping matches as a list.
pub fn find_all(args: &[Value], _named: &HashMap<String, Value>) -> CorvoResult<Value> {
    let (pattern, flags) = extract_regex(args.first())?;
    let text = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| {
            CorvoError::r#type("re.find_all requires a string as the second argument")
        })?;
    let re = build_regex(pattern, flags)?;
    let matches: Vec<Value> = re
        .find_iter(text)
        .map(|m| Value::String(m.as_str().to_string()))
        .collect();
    Ok(Value::List(matches))
}

/// `re.replace(regex, string, replacement)` – replaces the first match.
pub fn replace(args: &[Value], _named: &HashMap<String, Value>) -> CorvoResult<Value> {
    let (pattern, flags) = extract_regex(args.first())?;
    let text = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| CorvoError::r#type("re.replace requires a string as the second argument"))?;
    let replacement = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .unwrap_or("");
    let re = build_regex(pattern, flags)?;
    Ok(Value::String(re.replace(text, replacement).into_owned()))
}

/// `re.replace_all(regex, string, replacement)` – replaces all matches.
pub fn replace_all(args: &[Value], _named: &HashMap<String, Value>) -> CorvoResult<Value> {
    let (pattern, flags) = extract_regex(args.first())?;
    let text = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| {
            CorvoError::r#type("re.replace_all requires a string as the second argument")
        })?;
    let replacement = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .unwrap_or("");
    let re = build_regex(pattern, flags)?;
    Ok(Value::String(
        re.replace_all(text, replacement).into_owned(),
    ))
}

/// `re.split(regex, string)` – splits a string by the regex and returns a list.
pub fn split(args: &[Value], _named: &HashMap<String, Value>) -> CorvoResult<Value> {
    let (pattern, flags) = extract_regex(args.first())?;
    let text = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| CorvoError::r#type("re.split requires a string as the second argument"))?;
    let re = build_regex(pattern, flags)?;
    let parts: Vec<Value> = re
        .split(text)
        .map(|s| Value::String(s.to_string()))
        .collect();
    Ok(Value::List(parts))
}

/// `re.new(pattern)` or `re.new(pattern, flags)` – creates a new regex value.
pub fn new_regex(args: &[Value], _named: &HashMap<String, Value>) -> CorvoResult<Value> {
    let pattern = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| CorvoError::r#type("re.new requires a pattern string"))?;
    let flags = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .unwrap_or("");
    // Validate the pattern up-front.
    build_regex(pattern, flags)?;
    Ok(Value::Regex(pattern.to_string(), flags.to_string()))
}

fn posix_class_chars_vec(class: &str) -> CorvoResult<Vec<char>> {
    let chars: Vec<char> = match class {
        "graph" => (33u8..=126u8).map(char::from).collect(), // printable except space
        "print" => (32u8..=126u8).map(char::from).collect(), // printable including space
        "space" => vec![' ', '\t', '\n', '\r', '\u{000B}', '\u{000C}'],
        "upper" => ('A'..='Z').collect(),
        "lower" => ('a'..='z').collect(),
        _ => {
            return Err(CorvoError::invalid_argument(format!(
                "re.posix_class_chars: unsupported class '{class}'"
            )));
        }
    };
    Ok(chars)
}

/// Expand a POSIX character class token (e.g. `[:space:]`) to the concrete set.
/// Supported classes are ASCII-oriented to mirror GNU/POSIX `tr` byte semantics.
pub fn posix_class_chars(args: &[Value], _named: &HashMap<String, Value>) -> CorvoResult<Value> {
    let class = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| CorvoError::invalid_argument("re.posix_class_chars requires class name"))?;

    Ok(Value::String(
        posix_class_chars_vec(class)?.into_iter().collect(),
    ))
}

/// Translate characters in `text` from one POSIX class to another.
/// Similar to `tr '[:upper:]' '[:lower:]'` when used with `upper -> lower`.
pub fn posix_class_translate(
    args: &[Value],
    _named: &HashMap<String, Value>,
) -> CorvoResult<Value> {
    let text = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| CorvoError::invalid_argument("re.posix_class_translate requires text"))?;
    let from_class = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| {
            CorvoError::invalid_argument("re.posix_class_translate requires from_class")
        })?;
    let to_class = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .ok_or_else(|| {
            CorvoError::invalid_argument("re.posix_class_translate requires to_class")
        })?;

    let from = posix_class_chars_vec(from_class)?;
    let to = posix_class_chars_vec(to_class)?;
    if to.is_empty() {
        return Err(CorvoError::invalid_argument(
            "re.posix_class_translate: destination class is empty",
        ));
    }

    let mut map: HashMap<char, char> = HashMap::new();
    for (i, src) in from.iter().enumerate() {
        // GNU tr behavior: when destination set is shorter, last char is reused.
        let dst = to[i.min(to.len() - 1)];
        map.insert(*src, dst);
    }

    let out: String = text
        .chars()
        .map(|ch| map.get(&ch).copied().unwrap_or(ch))
        .collect();
    Ok(Value::String(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_args() -> HashMap<String, Value> {
        HashMap::new()
    }

    fn class_chars(name: &str) -> String {
        posix_class_chars(&[Value::String(name.to_string())], &no_args())
            .unwrap()
            .as_string()
            .unwrap()
            .clone()
    }

    #[test]
    fn posix_graph_excludes_space_print_includes_space() {
        let graph = class_chars("graph");
        let print = class_chars("print");
        assert!(!graph.contains(' '), "[:graph:] must exclude space");
        assert!(print.contains(' '), "[:print:] must include space");
        assert!(graph.contains('!') && graph.contains('~'));
        assert!(print.contains('!') && print.contains('~'));
    }

    #[test]
    fn posix_space_matches_only_whitespace_chars() {
        let space = class_chars("space");
        for ch in [' ', '\t', '\n', '\r', '\u{000B}', '\u{000C}'] {
            assert!(space.contains(ch), "[:space:] missing {:?}", ch);
        }
        for ch in ['A', 'z', '0', '_'] {
            assert!(!space.contains(ch), "[:space:] must not include {:?}", ch);
        }
    }

    #[test]
    fn posix_upper_and_lower_sets_are_correct() {
        let upper = class_chars("upper");
        let lower = class_chars("lower");
        assert!(upper.contains('A') && upper.contains('Z'));
        assert!(lower.contains('a') && lower.contains('z'));
        assert!(!upper.contains('a') && !lower.contains('A'));
    }

    #[test]
    fn posix_translate_upper_to_lower() {
        let out = posix_class_translate(
            &[
                Value::String("HELLO WORLD 123".to_string()),
                Value::String("upper".to_string()),
                Value::String("lower".to_string()),
            ],
            &no_args(),
        )
        .unwrap();
        assert_eq!(out, Value::String("hello world 123".to_string()));
    }

    #[test]
    fn posix_translate_lower_to_upper() {
        let out = posix_class_translate(
            &[
                Value::String("hello world".to_string()),
                Value::String("lower".to_string()),
                Value::String("upper".to_string()),
            ],
            &no_args(),
        )
        .unwrap();
        assert_eq!(out, Value::String("HELLO WORLD".to_string()));
    }

    /// Issue #23: when the destination class is shorter than the source class, `tr`-style
    /// behavior must reuse the *last* destination character for the leftover source positions.
    /// Source `upper` has 26 chars, dest `space` has 6 chars, so 'A'..'F' map to the 6 space
    /// chars in order, and 'G'..'Z' all map to the final space char (form feed, U+000C).
    #[test]
    fn posix_translate_reuses_last_dst_when_destination_is_shorter() {
        let out = posix_class_translate(
            &[
                Value::String("ABCDEFGZ".to_string()),
                Value::String("upper".to_string()),
                Value::String("space".to_string()),
            ],
            &no_args(),
        )
        .unwrap();
        // space class order: [' ', '\t', '\n', '\r', VT (\u{000B}), FF (\u{000C})]
        let expected: String = [
            ' ', '\t', '\n', '\r', '\u{000B}', '\u{000C}', '\u{000C}', '\u{000C}',
        ]
        .iter()
        .collect();
        assert_eq!(out, Value::String(expected));
    }

    /// Issue #23: characters that are not in the source class must be passed through unchanged.
    #[test]
    fn posix_translate_passes_through_unmapped_chars() {
        let out = posix_class_translate(
            &[
                Value::String("AbCd 12".to_string()),
                Value::String("upper".to_string()),
                Value::String("lower".to_string()),
            ],
            &no_args(),
        )
        .unwrap();
        // Only 'A' and 'C' are in upper; 'b','d',' ','1','2' must remain unchanged.
        assert_eq!(out, Value::String("abcd 12".to_string()));
    }

    /// Issue #23: an unknown POSIX class name must produce a clear error rather than silently
    /// returning an empty character set (which would corrupt `tr`-style translations).
    #[test]
    fn posix_class_chars_rejects_unknown_class() {
        let err = posix_class_chars(&[Value::String("digit".to_string())], &no_args()).unwrap_err();
        assert!(
            err.to_string().contains("unsupported class"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn posix_class_translate_rejects_unknown_from_class() {
        let err = posix_class_translate(
            &[
                Value::String("abc".to_string()),
                Value::String("digit".to_string()),
                Value::String("lower".to_string()),
            ],
            &no_args(),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("unsupported class"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn posix_class_translate_rejects_unknown_to_class() {
        let err = posix_class_translate(
            &[
                Value::String("abc".to_string()),
                Value::String("lower".to_string()),
                Value::String("alpha".to_string()),
            ],
            &no_args(),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("unsupported class"),
            "unexpected error: {err}"
        );
    }
}
