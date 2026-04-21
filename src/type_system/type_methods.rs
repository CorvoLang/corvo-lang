use crate::strverscmp::strverscmp;
use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use regex::Regex;
use std::cmp::Ordering;

fn cmp_values_for_sort(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        _ => a.to_string().cmp(&b.to_string()),
    }
}

/// Base32-family encode using a 32-byte alphabet slice.
fn base32_encode_alphabet(input: &[u8], alphabet: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < input.len() {
        let b = |j: usize| {
            if i + j < input.len() {
                input[i + j]
            } else {
                0u8
            }
        };
        out.push(alphabet[(b(0) >> 3) as usize] as char);
        out.push(alphabet[((b(0) & 0x07) << 2 | b(1) >> 6) as usize] as char);
        if i + 1 < input.len() {
            out.push(alphabet[((b(1) & 0x3e) >> 1) as usize] as char);
            out.push(alphabet[((b(1) & 0x01) << 4 | b(2) >> 4) as usize] as char);
        } else {
            out.push_str("==");
        }
        if i + 2 < input.len() {
            out.push(alphabet[((b(2) & 0x0f) << 1 | b(3) >> 7) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 3 < input.len() {
            out.push(alphabet[((b(3) & 0x7c) >> 2) as usize] as char);
            out.push(alphabet[((b(3) & 0x03) << 3 | b(4) >> 5) as usize] as char);
        } else {
            out.push_str("==");
        }
        if i + 4 < input.len() {
            out.push(alphabet[(b(4) & 0x1f) as usize] as char);
        } else {
            out.push('=');
        }
        i += 5;
    }
    out
}

/// Base32-family decode. `alphabet` contains valid chars (case-insensitive for standard,
/// `hex_mode` for base32hex where digits 0-9 precede letters).
fn base32_decode_alphabet(input: &str, _alphabet: &str, hex_mode: bool) -> Result<Vec<u8>, String> {
    let upper = input.to_uppercase();
    let cleaned: String = upper.chars().filter(|c| !c.is_whitespace()).collect();

    let val = |c: char| -> Option<u8> {
        if hex_mode {
            match c {
                '0'..='9' => Some(c as u8 - b'0'),
                'A'..='V' => Some(c as u8 - b'A' + 10),
                _ => None,
            }
        } else {
            match c {
                'A'..='Z' => Some(c as u8 - b'A'),
                '2'..='7' => Some(c as u8 - b'2' + 26),
                _ => None,
            }
        }
    };

    let mut out = Vec::new();
    let chars: Vec<char> = cleaned.chars().collect();
    let mut i = 0;
    while i + 7 < chars.len() || (i < chars.len() && chars.len() - i == 8) {
        if chars.len() < i + 8 {
            break;
        }
        let chunk = &chars[i..i + 8];
        let v: Vec<Option<u8>> = chunk.iter().map(|&c| val(c)).collect();
        if let (Some(c0), Some(c1)) = (v[0], v[1]) {
            out.push((c0 << 3) | (c1 >> 2));
        } else {
            break;
        }
        if chunk[2] != '=' {
            if let (Some(c1), Some(c2), Some(c3)) = (v[1], v[2], v[3]) {
                out.push((c1 << 6) | (c2 << 1) | (c3 >> 4));
            }
        }
        if chunk[4] != '=' {
            if let (Some(c3), Some(c4)) = (v[3], v[4]) {
                out.push((c3 << 4) | (c4 >> 1));
            }
        }
        if chunk[5] != '=' {
            if let (Some(c4), Some(c5), Some(c6)) = (v[4], v[5], v[6]) {
                out.push((c4 << 7) | (c5 << 2) | (c6 >> 3));
            }
        }
        if chunk[7] != '=' {
            if let (Some(c6), Some(c7)) = (v[6], v[7]) {
                out.push((c6 << 5) | c7);
            }
        }
        i += 8;
    }
    Ok(out)
}

pub fn call_string_method(name: &str, args: &[Value]) -> CorvoResult<Value> {
    let method = name.strip_prefix("string.").unwrap();
    let target = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::r#type("string method requires a string target"))?;

    match method {
        "concat" => {
            let s2 = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            Ok(Value::String(format!("{}{}", target, s2)))
        }
        "replace" => {
            let old = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            let new = args
                .get(2)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            Ok(Value::String(target.replace(old, new)))
        }
        "split" => {
            let delimiter = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            let parts: Vec<Value> = target
                .split(delimiter)
                .map(|s| Value::String(s.to_string()))
                .collect();
            Ok(Value::List(parts))
        }
        "trim" => Ok(Value::String(target.trim().to_string())),
        "contains" => {
            let substr = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            Ok(Value::Boolean(target.contains(substr)))
        }
        "starts_with" => {
            let prefix = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            Ok(Value::Boolean(target.starts_with(prefix)))
        }
        "ends_with" => {
            let suffix = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            Ok(Value::Boolean(target.ends_with(suffix)))
        }
        "to_lower" => Ok(Value::String(target.to_lowercase())),
        "to_upper" => Ok(Value::String(target.to_uppercase())),
        "len" => Ok(Value::Number(target.len() as f64)),
        "reverse" => Ok(Value::String(target.chars().rev().collect())),
        "is_empty" => Ok(Value::Boolean(target.is_empty())),
        "pad_start" => {
            let width = args.get(1).and_then(|v| v.as_number()).ok_or_else(|| {
                CorvoError::invalid_argument("string.pad_start requires width (number)")
            })? as usize;
            let fill_ch = args
                .get(2)
                .and_then(|v| v.as_string())
                .and_then(|s| s.chars().next())
                .unwrap_or(' ');
            let n = target.chars().count();
            if n >= width {
                Ok(Value::String(target.clone()))
            } else {
                let pad: String = std::iter::repeat_n(fill_ch, width - n).collect();
                Ok(Value::String(format!("{}{}", pad, target)))
            }
        }
        "pad_end" => {
            let width = args.get(1).and_then(|v| v.as_number()).ok_or_else(|| {
                CorvoError::invalid_argument("string.pad_end requires width (number)")
            })? as usize;
            let fill_ch = args
                .get(2)
                .and_then(|v| v.as_string())
                .and_then(|s| s.chars().next())
                .unwrap_or(' ');
            let n = target.chars().count();
            if n >= width {
                Ok(Value::String(target.clone()))
            } else {
                let pad: String = std::iter::repeat_n(fill_ch, width - n).collect();
                Ok(Value::String(format!("{}{}", target, pad)))
            }
        }
        "fnmatch" => {
            let pattern = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
                CorvoError::invalid_argument("string.fnmatch requires a glob pattern (string)")
            })?;
            let mut re = String::from("^");
            for ch in pattern.chars() {
                match ch {
                    '*' => re.push_str(".*"),
                    '?' => re.push('.'),
                    '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                        re.push('\\');
                        re.push(ch);
                    }
                    _ => re.push(ch),
                }
            }
            re.push('$');
            let regex = Regex::new(&re)
                .map_err(|e| CorvoError::runtime(format!("invalid glob pattern: {}", e)))?;
            Ok(Value::Boolean(regex.is_match(target)))
        }
        "byte_slice" => {
            // args[0] is the target string (bound above), args[1] = start byte index,
            // args[2] = end byte index (optional, defaults to string length).
            let bytes = target.as_bytes();
            let start = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
            let end = args
                .get(2)
                .and_then(|v| v.as_number())
                .map(|n| n as usize)
                .unwrap_or(bytes.len());
            let end = end.min(bytes.len());
            let start = start.min(end);
            Ok(Value::String(
                String::from_utf8_lossy(&bytes[start..end]).into_owned(),
            ))
        }
        "trim_start" => Ok(Value::String(target.trim_start().to_string())),
        "repeat" => {
            let count = args.get(1).and_then(|v| v.as_number()).ok_or_else(|| {
                CorvoError::invalid_argument("string.repeat requires a count (number)")
            })? as usize;
            Ok(Value::String(target.repeat(count)))
        }
        "trim_end" => Ok(Value::String(target.trim_end().to_string())),
        "substring" => {
            // string.substring(s, start, end?) — Unicode-character-aware slice.
            // start is inclusive, end is exclusive and defaults to string length.
            let chars: Vec<char> = target.chars().collect();
            let len = chars.len();
            let start = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
            let end = args
                .get(2)
                .and_then(|v| v.as_number())
                .map(|n| n as usize)
                .unwrap_or(len);
            let start = start.min(len);
            let end = end.min(len);
            let start = start.min(end);
            Ok(Value::String(chars[start..end].iter().collect()))
        }
        "index_of" => {
            // string.index_of(s, needle, start?) — first char-index of needle, or -1.
            let needle = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            let start_char = args.get(2).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
            if needle.is_empty() {
                return Ok(Value::Number(start_char as f64));
            }
            // Convert char-start to byte-start.
            let byte_offset: usize = target
                .char_indices()
                .nth(start_char)
                .map(|(b, _)| b)
                .unwrap_or(target.len());
            let slice = &target[byte_offset..];
            match slice.find(needle) {
                Some(byte_pos) => {
                    // byte_pos is relative to slice; convert to char index in full string.
                    let abs_byte = byte_offset + byte_pos;
                    let char_idx = target[..abs_byte].chars().count();
                    Ok(Value::Number(char_idx as f64))
                }
                None => Ok(Value::Number(-1.0)),
            }
        }
        "last_index_of" => {
            // string.last_index_of(s, needle) — last char-index of needle, or -1.
            let needle = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            if needle.is_empty() {
                return Ok(Value::Number(target.chars().count() as f64));
            }
            match target.rfind(needle) {
                Some(byte_pos) => {
                    let char_idx = target[..byte_pos].chars().count();
                    Ok(Value::Number(char_idx as f64))
                }
                None => Ok(Value::Number(-1.0)),
            }
        }
        "char_at" => {
            // string.char_at(s, index) — Unicode character at index, or error if out of range.
            let idx = args.get(1).and_then(|v| v.as_number()).ok_or_else(|| {
                CorvoError::invalid_argument("string.char_at requires an index (number)")
            })? as usize;
            target
                .chars()
                .nth(idx)
                .map(|c| Value::String(c.to_string()))
                .ok_or_else(|| {
                    CorvoError::runtime("string.char_at: index out of bounds".to_string())
                })
        }
        "replace_first" => {
            // string.replace_first(s, old, new) — replace only the first occurrence.
            let old = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            let new = args
                .get(2)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            Ok(Value::String(target.replacen(old, new, 1)))
        }
        "count" => {
            // string.count(s, needle) — count non-overlapping occurrences of needle.
            let needle = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            if needle.is_empty() {
                // Matches Python/JS: empty needle returns char_count + 1.
                return Ok(Value::Number((target.chars().count() + 1) as f64));
            }
            Ok(Value::Number(target.matches(needle).count() as f64))
        }
        "chars" => {
            // string.chars(s) — split into a list of individual Unicode characters.
            let list: Vec<Value> = target
                .chars()
                .map(|c| Value::String(c.to_string()))
                .collect();
            Ok(Value::List(list))
        }
        "base64_encode" => {
            use base64::Engine as _;
            Ok(Value::String(
                base64::engine::general_purpose::STANDARD.encode(target.as_bytes()),
            ))
        }
        "base64_decode" => {
            use base64::Engine as _;
            let cleaned: String = target.chars().filter(|c| !c.is_whitespace()).collect();
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(cleaned.as_bytes())
                .map_err(|e| CorvoError::invalid_argument(format!("base64_decode: {}", e)))?;
            String::from_utf8(bytes).map(Value::String).map_err(|e| {
                CorvoError::invalid_argument(format!("base64_decode: not valid UTF-8: {}", e))
            })
        }
        "base32_encode" => Ok(Value::String(base32_encode_alphabet(
            target.as_bytes(),
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
        ))),
        "base32_decode" => {
            let bytes = base32_decode_alphabet(
                target,
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz234567",
                false,
            )
            .map_err(|e| CorvoError::invalid_argument(format!("base32_decode: {}", e)))?;
            String::from_utf8(bytes).map(Value::String).map_err(|e| {
                CorvoError::invalid_argument(format!("base32_decode: not valid UTF-8: {}", e))
            })
        }
        "base32hex_encode" => Ok(Value::String(base32_encode_alphabet(
            target.as_bytes(),
            b"0123456789ABCDEFGHIJKLMNOPQRSTUV",
        ))),
        "base32hex_decode" => {
            let bytes = base32_decode_alphabet(
                target,
                "0123456789ABCDEFGHIJKLMNOPQRSTUVabcdefghijklmnopqrstuv",
                true,
            )
            .map_err(|e| CorvoError::invalid_argument(format!("base32hex_decode: {}", e)))?;
            String::from_utf8(bytes).map(Value::String).map_err(|e| {
                CorvoError::invalid_argument(format!("base32hex_decode: not valid UTF-8: {}", e))
            })
        }
        "hex_encode" => {
            let hex: String = target.bytes().map(|b| format!("{:02x}", b)).collect();
            Ok(Value::String(hex))
        }
        "hex_decode" => {
            let cleaned: String = target.chars().filter(|c| !c.is_whitespace()).collect();
            if !cleaned.len().is_multiple_of(2) {
                return Err(CorvoError::invalid_argument(
                    "hex_decode: odd number of hex digits",
                ));
            }
            let bytes: Result<Vec<u8>, _> = (0..cleaned.len() / 2)
                .map(|i| u8::from_str_radix(&cleaned[i * 2..i * 2 + 2], 16))
                .collect();
            let bytes =
                bytes.map_err(|e| CorvoError::invalid_argument(format!("hex_decode: {}", e)))?;
            String::from_utf8(bytes).map(Value::String).map_err(|e| {
                CorvoError::invalid_argument(format!("hex_decode: not valid UTF-8: {}", e))
            })
        }
        _ => Err(CorvoError::unknown_function(format!("string.{}", method))),
    }
}

pub fn call_number_method(name: &str, args: &[Value]) -> CorvoResult<Value> {
    let method = name.strip_prefix("number.").unwrap();
    let target = args.first().and_then(|v| v.as_number()).unwrap_or(0.0);

    match method {
        "to_string" => Ok(Value::String(target.to_string())),
        "parse" => {
            let s = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            s.parse::<f64>()
                .map(Value::Number)
                .map_err(|_| CorvoError::runtime("Failed to parse number".to_string()))
        }
        "is_nan" => Ok(Value::Boolean(target.is_nan())),
        "is_infinite" => Ok(Value::Boolean(target.is_infinite())),
        "is_finite" => Ok(Value::Boolean(target.is_finite())),
        "abs" => Ok(Value::Number(target.abs())),
        "floor" => Ok(Value::Number(target.floor())),
        "ceil" => Ok(Value::Number(target.ceil())),
        "round" => Ok(Value::Number(target.round())),
        "sqrt" => {
            if target < 0.0 {
                return Err(CorvoError::runtime(
                    "Cannot take square root of negative number".to_string(),
                ));
            }
            Ok(Value::Number(target.sqrt()))
        }
        _ => Err(CorvoError::unknown_function(format!("number.{}", method))),
    }
}

/// GNU `ls -C` output: coreutils `print_many_per_line` with `calculate_columns(true)`.
fn gnu_ls_column_format(names: &[String], line_length: usize, tabsize: usize) -> String {
    const MIN_COLUMN_WIDTH: usize = 3;
    let n = names.len();
    if n == 0 {
        return String::new();
    }
    let lens: Vec<usize> = names.iter().map(|s| s.chars().count()).collect();

    let max_idx =
        line_length / MIN_COLUMN_WIDTH + usize::from(!line_length.is_multiple_of(MIN_COLUMN_WIDTH));
    let max_cols = if max_idx > 0 && max_idx < n {
        max_idx
    } else {
        n
    };

    #[derive(Clone)]
    struct ColInfo {
        valid_len: bool,
        line_len: usize,
        col_arr: Vec<usize>,
    }

    let mut column_info: Vec<ColInfo> = (0..max_cols)
        .map(|i| {
            let cols = i + 1;
            ColInfo {
                valid_len: true,
                line_len: cols * MIN_COLUMN_WIDTH,
                col_arr: vec![MIN_COLUMN_WIDTH; cols],
            }
        })
        .collect();

    for (filesno, name_length) in lens.iter().copied().enumerate() {
        for (i, ci) in column_info.iter_mut().enumerate().take(max_cols) {
            if ci.valid_len {
                let denom = (n + i) / (i + 1);
                let idx = filesno / denom;
                let real_length = name_length + if idx == i { 0 } else { 2 };
                if ci.col_arr[idx] < real_length {
                    ci.line_len += real_length - ci.col_arr[idx];
                    ci.col_arr[idx] = real_length;
                    ci.valid_len = ci.line_len < line_length;
                }
            }
        }
    }

    let mut cols = max_cols;
    while cols > 1 && !column_info[cols - 1].valid_len {
        cols -= 1;
    }

    let line_fmt = &column_info[cols - 1];
    let rows = n / cols + usize::from(!n.is_multiple_of(cols));

    let mut line_strs: Vec<String> = Vec::with_capacity(rows);
    for row in 0..rows {
        let mut col_ix = 0usize;
        let mut filesno = row;
        let mut pos = 0usize;
        let mut line = String::new();
        loop {
            let name_length = lens[filesno];
            let max_name_length = line_fmt.col_arr[col_ix];
            line.push_str(&names[filesno]);
            col_ix += 1;
            if n - rows <= filesno {
                break;
            }
            filesno += rows;
            let mut from = pos + name_length;
            let to = pos + max_name_length;
            while from < to {
                if tabsize != 0 && to / tabsize > (from + 1) / tabsize {
                    line.push('\t');
                    from += tabsize - from % tabsize;
                } else {
                    line.push(' ');
                    from += 1;
                }
            }
            pos += max_name_length;
        }
        line_strs.push(line);
    }
    line_strs.join("\n")
}

pub fn call_list_method(name: &str, args: &[Value]) -> CorvoResult<Value> {
    let method = name.strip_prefix("list.").unwrap();
    let target = args
        .first()
        .and_then(|v| v.as_list())
        .cloned()
        .unwrap_or_default();

    match method {
        "push" => {
            let item = args.get(1).cloned().unwrap_or(Value::Null);
            let mut new_list = target.clone();
            new_list.push(item);
            Ok(Value::List(new_list))
        }
        "pop" => {
            let mut new_list = target.clone();
            new_list.pop();
            Ok(Value::List(new_list))
        }
        "get" => {
            let index = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
            target
                .get(index)
                .cloned()
                .ok_or_else(|| CorvoError::runtime("Index out of bounds".to_string()))
        }
        "set" => {
            let index = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
            let value = args.get(2).cloned().unwrap_or(Value::Null);
            if index >= target.len() {
                return Err(CorvoError::runtime("Index out of bounds".to_string()));
            }
            let mut new_list = target.clone();
            new_list[index] = value;
            Ok(Value::List(new_list))
        }
        "concat" => {
            let mut new_list = target.clone();
            for arg in args.iter().skip(1) {
                if let Value::List(inner) = arg {
                    new_list.extend(inner.iter().cloned());
                } else {
                    new_list.push(arg.clone());
                }
            }
            Ok(Value::List(new_list))
        }
        "first" => target
            .first()
            .cloned()
            .ok_or_else(|| CorvoError::runtime("List is empty".to_string())),
        "last" => target
            .last()
            .cloned()
            .ok_or_else(|| CorvoError::runtime("List is empty".to_string())),
        "len" => Ok(Value::Number(target.len() as f64)),
        "reverse" => {
            let mut new_list = target.clone();
            new_list.reverse();
            Ok(Value::List(new_list))
        }
        "is_empty" => Ok(Value::Boolean(target.is_empty())),
        "contains" => {
            let item = args.get(1).cloned().unwrap_or(Value::Null);
            Ok(Value::Boolean(target.contains(&item)))
        }
        "join" => {
            let delimiter = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            let parts: Vec<String> = target.iter().map(|v| v.to_string()).collect();
            Ok(Value::String(parts.join(delimiter)))
        }
        "new" => Ok(Value::List(Vec::new())),
        "delete" => {
            let index = args.get(1).and_then(|v| v.as_number()).ok_or_else(|| {
                CorvoError::runtime("list.delete requires an index argument".to_string())
            })? as usize;
            if index >= target.len() {
                return Err(CorvoError::runtime("Index out of bounds".to_string()));
            }
            let mut new_list = target.clone();
            new_list.remove(index);
            Ok(Value::List(new_list))
        }
        "sort" => {
            let mut new_list = target.clone();
            new_list.sort_by_key(|a| a.to_string());
            Ok(Value::List(new_list))
        }
        "sort_version" => {
            let mut new_list = target.clone();
            new_list.sort_by(|a, b| strverscmp(&a.to_string(), &b.to_string()));
            Ok(Value::List(new_list))
        }
        "sort_maps_by_key" => {
            let key = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    CorvoError::invalid_argument(
                        "list.sort_maps_by_key requires key name (string) as second argument",
                    )
                })?;
            let reverse = args.get(2).and_then(|v| v.as_bool()).unwrap_or(false);
            let tie_key = args
                .get(3)
                .and_then(|v| v.as_string())
                .map(|s| s.to_string());
            let secondary_key = args
                .get(4)
                .and_then(|v| v.as_string())
                .map(|s| s.to_string());
            let mut new_list = target.clone();
            new_list.sort_by(|a, b| {
                let va = a
                    .as_map()
                    .and_then(|m| m.get(&key))
                    .cloned()
                    .unwrap_or(Value::Null);
                let vb = b
                    .as_map()
                    .and_then(|m| m.get(&key))
                    .cloned()
                    .unwrap_or(Value::Null);
                let ord = cmp_values_for_sort(&va, &vb);
                let ord = if reverse { ord.reverse() } else { ord };
                if ord != Ordering::Equal {
                    return ord;
                }
                if let Some(ref sk) = secondary_key {
                    let sa = a
                        .as_map()
                        .and_then(|m| m.get(sk))
                        .cloned()
                        .unwrap_or(Value::Null);
                    let sb = b
                        .as_map()
                        .and_then(|m| m.get(sk))
                        .cloned()
                        .unwrap_or(Value::Null);
                    let ord_s = cmp_values_for_sort(&sa, &sb);
                    let ord_s = if reverse { ord_s.reverse() } else { ord_s };
                    if ord_s != Ordering::Equal {
                        return ord_s;
                    }
                }
                if let Some(ref tk) = tie_key {
                    let ta = a
                        .as_map()
                        .and_then(|m| m.get(tk))
                        .cloned()
                        .unwrap_or(Value::Null);
                    let tb = b
                        .as_map()
                        .and_then(|m| m.get(tk))
                        .cloned()
                        .unwrap_or(Value::Null);
                    // GNU ls uses ascending name (etc.) as the final tie-break regardless of -r.
                    cmp_values_for_sort(&ta, &tb)
                } else {
                    Ordering::Equal
                }
            });
            Ok(Value::List(new_list))
        }
        "columnate" => {
            let width = args
                .get(1)
                .and_then(|v| v.as_number())
                .unwrap_or(80.0)
                .max(1.0) as usize;
            let tabsize = args
                .get(2)
                .and_then(|v| v.as_number())
                .unwrap_or(8.0)
                .max(0.0) as usize;
            let names: Vec<String> = target
                .iter()
                .filter_map(|v| v.as_string().cloned())
                .collect();
            Ok(Value::String(gnu_ls_column_format(&names, width, tabsize)))
        }
        "find" => {
            let item = args.get(1).cloned().unwrap_or(Value::Null);
            let index = target.iter().position(|v| v == &item);
            Ok(index
                .map(|i| Value::Number(i as f64))
                .unwrap_or(Value::Number(-1.0)))
        }
        "slice" => {
            let start = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as usize;
            let end = args
                .get(2)
                .and_then(|v| v.as_number())
                .map(|n| n as usize)
                .unwrap_or(target.len());
            let end = end.min(target.len());
            let start = start.min(end);
            Ok(Value::List(target[start..end].to_vec()))
        }
        "unique" => {
            let mut seen = std::collections::HashSet::new();
            let new_list: Vec<Value> = target
                .iter()
                .filter(|v| seen.insert(v.to_string()))
                .cloned()
                .collect();
            Ok(Value::List(new_list))
        }
        "flatten" => {
            let mut new_list = Vec::new();
            for item in &target {
                if let Value::List(inner) = item {
                    new_list.extend(inner.iter().cloned());
                } else {
                    new_list.push(item.clone());
                }
            }
            Ok(Value::List(new_list))
        }
        _ => Err(CorvoError::unknown_function(format!("list.{}", method))),
    }
}

pub fn call_map_method(name: &str, args: &[Value]) -> CorvoResult<Value> {
    let method = name.strip_prefix("map.").unwrap();
    let target = args
        .first()
        .and_then(|v| v.as_map())
        .cloned()
        .unwrap_or_default();

    match method {
        "keys" => {
            let keys: Vec<Value> = target.keys().map(|k| Value::String(k.clone())).collect();
            Ok(Value::List(keys))
        }
        "values" => Ok(Value::List(target.values().cloned().collect())),
        "len" => Ok(Value::Number(target.len() as f64)),
        "is_empty" => Ok(Value::Boolean(target.is_empty())),
        "has_key" => {
            let key = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            Ok(Value::Boolean(target.contains_key(key)))
        }
        "get" => {
            let key = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            let default = args.get(2).cloned().unwrap_or(Value::Null);
            Ok(target.get(key).cloned().unwrap_or(default))
        }
        "set" => {
            let key = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let value = args.get(2).cloned().unwrap_or(Value::Null);
            let mut new_map = target.clone();
            new_map.insert(key, value);
            Ok(Value::Map(new_map))
        }
        "remove" | "delete" => {
            let key = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            let mut new_map = target.clone();
            new_map.remove(key);
            Ok(Value::Map(new_map))
        }
        "merge" => {
            let other = args
                .get(1)
                .and_then(|v| v.as_map())
                .cloned()
                .unwrap_or_default();
            let mut new_map = target.clone();
            new_map.extend(other);
            Ok(Value::Map(new_map))
        }
        "has" => {
            let key = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.as_str())
                .unwrap_or("");
            Ok(Value::Boolean(target.contains_key(key)))
        }
        "entries" => {
            let mut entries: Vec<Value> = target
                .iter()
                .map(|(k, v)| {
                    let mut entry = std::collections::HashMap::new();
                    entry.insert("key".to_string(), Value::String(k.clone()));
                    entry.insert("value".to_string(), v.clone());
                    Value::Map(entry)
                })
                .collect();
            entries.sort_by(|a, b| {
                let ka = if let Value::Map(m) = a {
                    m.get("key").map(|v| v.to_string()).unwrap_or_default()
                } else {
                    String::new()
                };
                let kb = if let Value::Map(m) = b {
                    m.get("key").map(|v| v.to_string()).unwrap_or_default()
                } else {
                    String::new()
                };
                ka.cmp(&kb)
            });
            Ok(Value::List(entries))
        }
        "new" => Ok(Value::Map(std::collections::HashMap::new())),
        "column" => map_column(&target, args),
        _ => Err(CorvoError::unknown_function(format!("map.{}", method))),
    }
}

fn map_cell_width(s: &str) -> usize {
    s.chars().count()
}

fn map_pad_cell(s: &str, width: usize) -> String {
    let w = map_cell_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - w))
    }
}

fn map_format_row(cells: &[String], widths: &[usize], gap: &str) -> String {
    let parts: Vec<String> = cells
        .iter()
        .enumerate()
        .map(|(i, c)| map_pad_cell(c, widths[i]))
        .collect();
    parts.join(gap)
}

/// Formats a map as plain-text columns (similar to the `column` utility).
///
/// If every value is a list, keys are column headers and each list is one column (all lists must be the same length). Otherwise keys and values are printed as two aligned columns. Optional second argument: gap width in spaces between columns (default 2, max 64).
fn map_column(
    target: &std::collections::HashMap<String, Value>,
    args: &[Value],
) -> CorvoResult<Value> {
    if target.is_empty() {
        return Ok(Value::String(String::new()));
    }

    let gap_n = args
        .get(1)
        .and_then(|v| v.as_number())
        .map(|n| n.clamp(1.0, 64.0) as usize)
        .unwrap_or(2);
    let gap = " ".repeat(gap_n);

    let mut keys: Vec<&String> = target.keys().collect();
    keys.sort();

    let all_lists = keys
        .iter()
        .all(|k| matches!(target.get(*k).unwrap_or(&Value::Null), Value::List(_)));
    let any_list = keys
        .iter()
        .any(|k| matches!(target.get(*k).unwrap_or(&Value::Null), Value::List(_)));

    if any_list && !all_lists {
        return Err(CorvoError::invalid_argument(
            "map.column: either all values must be lists (same length per column) or none may be lists (key/value table)",
        ));
    }

    if all_lists {
        let row_counts: Vec<usize> = keys
            .iter()
            .map(|k| {
                target
                    .get(*k)
                    .and_then(|v| v.as_list())
                    .map(|l| l.len())
                    .unwrap_or(0)
            })
            .collect();
        let nrows = *row_counts.first().unwrap_or(&0);
        if row_counts.iter().any(|&c| c != nrows) {
            return Err(CorvoError::invalid_argument(
                "map.column: all list values must have the same length",
            ));
        }

        let ncol = keys.len();
        let mut widths = vec![0usize; ncol];
        for (j, k) in keys.iter().enumerate() {
            widths[j] = widths[j].max(map_cell_width(k.as_str()));
        }
        let mut grid: Vec<Vec<String>> = vec![vec![String::new(); ncol]; nrows];
        for (j, k) in keys.iter().enumerate() {
            let list = target.get(*k).unwrap().as_list().unwrap();
            for (row, v) in list.iter().enumerate() {
                let cell = v.to_string();
                widths[j] = widths[j].max(map_cell_width(&cell));
                grid[row][j] = cell;
            }
        }

        let header_cells: Vec<String> = keys.iter().map(|k| (*k).clone()).collect();
        let mut lines = vec![map_format_row(&header_cells, &widths, &gap)];
        for row_cells in grid {
            lines.push(map_format_row(&row_cells, &widths, &gap));
        }
        return Ok(Value::String(lines.join("\n")));
    }

    let mut kw = 0usize;
    let mut vw = 0usize;
    let rows: Vec<(String, String)> = keys
        .iter()
        .map(|k| {
            let val = target.get(*k).unwrap();
            let vs = val.to_string();
            kw = kw.max(map_cell_width(k.as_str()));
            vw = vw.max(map_cell_width(&vs));
            ((*k).clone(), vs)
        })
        .collect();

    let lines: Vec<String> = rows
        .iter()
        .map(|(k, v)| format!("{}{}{}", map_pad_cell(k, kw), gap, map_pad_cell(v, vw)))
        .collect();
    Ok(Value::String(lines.join("\n")))
}

pub fn call_re_method(name: &str, args: &[Value]) -> CorvoResult<Value> {
    let method = name.strip_prefix("re.").unwrap();
    let named = std::collections::HashMap::new();
    match method {
        "match" => crate::standard_lib::re::is_match(args, &named),
        "find" => crate::standard_lib::re::find(args, &named),
        "find_all" => crate::standard_lib::re::find_all(args, &named),
        "replace" => crate::standard_lib::re::replace(args, &named),
        "replace_all" => crate::standard_lib::re::replace_all(args, &named),
        "split" => crate::standard_lib::re::split(args, &named),
        "new" => crate::standard_lib::re::new_regex(args, &named),
        _ => Err(CorvoError::unknown_function(format!("re.{}", method))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_concat() {
        let args = vec![
            Value::String("hello".to_string()),
            Value::String(" world".to_string()),
        ];
        let result = call_string_method("string.concat", &args).unwrap();
        assert_eq!(result, Value::String("hello world".to_string()));
    }

    #[test]
    fn test_string_replace() {
        let args = vec![
            Value::String("hello world".to_string()),
            Value::String("world".to_string()),
            Value::String("rust".to_string()),
        ];
        let result = call_string_method("string.replace", &args).unwrap();
        assert_eq!(result, Value::String("hello rust".to_string()));
    }

    #[test]
    fn test_string_split() {
        let args = vec![
            Value::String("a,b,c".to_string()),
            Value::String(",".to_string()),
        ];
        let result = call_string_method("string.split", &args).unwrap();
        match result {
            Value::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], Value::String("a".to_string()));
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_string_trim() {
        let args = vec![Value::String("  hello  ".to_string())];
        let result = call_string_method("string.trim", &args).unwrap();
        assert_eq!(result, Value::String("hello".to_string()));
    }

    #[test]
    fn test_string_contains() {
        let args = vec![
            Value::String("hello world".to_string()),
            Value::String("world".to_string()),
        ];
        let result = call_string_method("string.contains", &args).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_string_to_lower() {
        let args = vec![Value::String("HELLO".to_string())];
        let result = call_string_method("string.to_lower", &args).unwrap();
        assert_eq!(result, Value::String("hello".to_string()));
    }

    #[test]
    fn test_string_to_upper() {
        let args = vec![Value::String("hello".to_string())];
        let result = call_string_method("string.to_upper", &args).unwrap();
        assert_eq!(result, Value::String("HELLO".to_string()));
    }

    #[test]
    fn test_string_len() {
        let args = vec![Value::String("hello".to_string())];
        let result = call_string_method("string.len", &args).unwrap();
        assert_eq!(result, Value::Number(5.0));
    }

    #[test]
    fn test_number_to_string() {
        let args = vec![Value::Number(42.5)];
        let result = call_number_method("number.to_string", &args).unwrap();
        assert_eq!(result, Value::String("42.5".to_string()));
    }

    #[test]
    fn test_number_parse() {
        let args = vec![Value::Number(0.0), Value::String("1.5".to_string())];
        let result = call_number_method("number.parse", &args).unwrap();
        assert_eq!(result, Value::Number(1.5));
    }

    #[test]
    fn test_number_is_nan() {
        let args = vec![Value::Number(f64::NAN)];
        let result = call_number_method("number.is_nan", &args).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_list_push() {
        let args = vec![
            Value::List(vec![Value::Number(1.0), Value::Number(2.0)]),
            Value::Number(3.0),
        ];
        let result = call_list_method("list.push", &args).unwrap();
        match result {
            Value::List(items) => assert_eq!(items.len(), 3),
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_list_pop() {
        let args = vec![Value::List(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ])];
        let result = call_list_method("list.pop", &args).unwrap();
        match result {
            Value::List(items) => assert_eq!(items.len(), 2),
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_list_get() {
        let args = vec![
            Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ]),
            Value::Number(1.0),
        ];
        let result = call_list_method("list.get", &args).unwrap();
        assert_eq!(result, Value::String("b".to_string()));
    }

    #[test]
    fn test_list_len() {
        let args = vec![Value::List(vec![Value::Number(1.0), Value::Number(2.0)])];
        let result = call_list_method("list.len", &args).unwrap();
        assert_eq!(result, Value::Number(2.0));
    }

    #[test]
    fn test_list_contains() {
        let args = vec![
            Value::List(vec![Value::Number(1.0), Value::Number(2.0)]),
            Value::Number(2.0),
        ];
        let result = call_list_method("list.contains", &args).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_list_join() {
        let args = vec![
            Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ]),
            Value::String(", ".to_string()),
        ];
        let result = call_list_method("list.join", &args).unwrap();
        assert_eq!(result, Value::String("a, b, c".to_string()));
    }

    #[test]
    fn test_map_keys() {
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), Value::Number(1.0));
        map.insert("b".to_string(), Value::Number(2.0));
        let args = vec![Value::Map(map)];
        let result = call_map_method("map.keys", &args).unwrap();
        match result {
            Value::List(keys) => assert_eq!(keys.len(), 2),
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_map_has_key() {
        let mut map = std::collections::HashMap::new();
        map.insert("key".to_string(), Value::Number(1.0));
        let args = vec![Value::Map(map), Value::String("key".to_string())];
        let result = call_map_method("map.has_key", &args).unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_map_get() {
        let mut map = std::collections::HashMap::new();
        map.insert("key".to_string(), Value::Number(42.0));
        let args = vec![Value::Map(map), Value::String("key".to_string())];
        let result = call_map_method("map.get", &args).unwrap();
        assert_eq!(result, Value::Number(42.0));
    }

    #[test]
    fn test_map_set() {
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), Value::Number(1.0));
        let args = vec![
            Value::Map(map),
            Value::String("b".to_string()),
            Value::Number(2.0),
        ];
        let result = call_map_method("map.set", &args).unwrap();
        match result {
            Value::Map(m) => assert_eq!(m.len(), 2),
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_unknown_string_method() {
        let args = vec![Value::String("test".to_string())];
        let result = call_string_method("string.unknown", &args);
        assert!(result.is_err());
    }

    // --- New String Method Tests ---

    #[test]
    fn test_string_starts_with() {
        let args = vec![
            Value::String("hello world".to_string()),
            Value::String("hello".to_string()),
        ];
        assert_eq!(
            call_string_method("string.starts_with", &args).unwrap(),
            Value::Boolean(true)
        );

        let args = vec![
            Value::String("hello world".to_string()),
            Value::String("world".to_string()),
        ];
        assert_eq!(
            call_string_method("string.starts_with", &args).unwrap(),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_string_ends_with() {
        let args = vec![
            Value::String("hello world".to_string()),
            Value::String("world".to_string()),
        ];
        assert_eq!(
            call_string_method("string.ends_with", &args).unwrap(),
            Value::Boolean(true)
        );
    }

    #[test]
    fn test_string_reverse() {
        let args = vec![Value::String("hello".to_string())];
        assert_eq!(
            call_string_method("string.reverse", &args).unwrap(),
            Value::String("olleh".to_string())
        );
    }

    #[test]
    fn test_string_is_empty() {
        assert_eq!(
            call_string_method("string.is_empty", &[Value::String("".to_string())]).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            call_string_method("string.is_empty", &[Value::String("a".to_string())]).unwrap(),
            Value::Boolean(false)
        );
    }

    // --- New String Method Tests ---

    #[test]
    fn test_string_trim_start() {
        assert_eq!(
            call_string_method(
                "string.trim_start",
                &[Value::String("  hello  ".to_string())]
            )
            .unwrap(),
            Value::String("hello  ".to_string())
        );
        assert_eq!(
            call_string_method("string.trim_start", &[Value::String("hello".to_string())]).unwrap(),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_string_trim_end() {
        assert_eq!(
            call_string_method("string.trim_end", &[Value::String("  hello  ".to_string())])
                .unwrap(),
            Value::String("  hello".to_string())
        );
        assert_eq!(
            call_string_method("string.trim_end", &[Value::String("hello".to_string())]).unwrap(),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_string_substring() {
        // Basic substring
        assert_eq!(
            call_string_method(
                "string.substring",
                &[
                    Value::String("hello world".to_string()),
                    Value::Number(6.0),
                    Value::Number(11.0),
                ]
            )
            .unwrap(),
            Value::String("world".to_string())
        );
        // Omit end → to end of string
        assert_eq!(
            call_string_method(
                "string.substring",
                &[Value::String("hello world".to_string()), Value::Number(6.0),]
            )
            .unwrap(),
            Value::String("world".to_string())
        );
        // Out-of-range end clamps to len
        assert_eq!(
            call_string_method(
                "string.substring",
                &[
                    Value::String("hi".to_string()),
                    Value::Number(0.0),
                    Value::Number(100.0),
                ]
            )
            .unwrap(),
            Value::String("hi".to_string())
        );
    }

    #[test]
    fn test_string_index_of() {
        assert_eq!(
            call_string_method(
                "string.index_of",
                &[
                    Value::String("hello world".to_string()),
                    Value::String("world".to_string()),
                ]
            )
            .unwrap(),
            Value::Number(6.0)
        );
        // Not found returns -1
        assert_eq!(
            call_string_method(
                "string.index_of",
                &[
                    Value::String("hello".to_string()),
                    Value::String("xyz".to_string()),
                ]
            )
            .unwrap(),
            Value::Number(-1.0)
        );
        // Start offset
        assert_eq!(
            call_string_method(
                "string.index_of",
                &[
                    Value::String("aabaa".to_string()),
                    Value::String("a".to_string()),
                    Value::Number(2.0),
                ]
            )
            .unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn test_string_char_at() {
        assert_eq!(
            call_string_method(
                "string.char_at",
                &[Value::String("hello".to_string()), Value::Number(1.0),]
            )
            .unwrap(),
            Value::String("e".to_string())
        );
        // Out-of-range is an error
        assert!(call_string_method(
            "string.char_at",
            &[Value::String("hi".to_string()), Value::Number(10.0),]
        )
        .is_err());
    }

    #[test]
    fn test_string_repeat() {
        assert_eq!(
            call_string_method(
                "string.repeat",
                &[Value::String("ab".to_string()), Value::Number(3.0),]
            )
            .unwrap(),
            Value::String("ababab".to_string())
        );
        assert_eq!(
            call_string_method(
                "string.repeat",
                &[Value::String("x".to_string()), Value::Number(0.0),]
            )
            .unwrap(),
            Value::String(String::new())
        );
    }

    #[test]
    fn test_string_replace_first() {
        assert_eq!(
            call_string_method(
                "string.replace_first",
                &[
                    Value::String("aabbaa".to_string()),
                    Value::String("a".to_string()),
                    Value::String("z".to_string()),
                ]
            )
            .unwrap(),
            Value::String("zabbaa".to_string())
        );
        // No match: unchanged
        assert_eq!(
            call_string_method(
                "string.replace_first",
                &[
                    Value::String("hello".to_string()),
                    Value::String("x".to_string()),
                    Value::String("y".to_string()),
                ]
            )
            .unwrap(),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_string_count() {
        assert_eq!(
            call_string_method(
                "string.count",
                &[
                    Value::String("aabbaa".to_string()),
                    Value::String("a".to_string()),
                ]
            )
            .unwrap(),
            Value::Number(4.0)
        );
        assert_eq!(
            call_string_method(
                "string.count",
                &[
                    Value::String("hello".to_string()),
                    Value::String("x".to_string()),
                ]
            )
            .unwrap(),
            Value::Number(0.0)
        );
        // Non-overlapping
        assert_eq!(
            call_string_method(
                "string.count",
                &[
                    Value::String("aaa".to_string()),
                    Value::String("aa".to_string()),
                ]
            )
            .unwrap(),
            Value::Number(1.0)
        );
    }

    #[test]
    fn test_string_chars() {
        let result =
            call_string_method("string.chars", &[Value::String("hi!".to_string())]).unwrap();
        match result {
            Value::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], Value::String("h".to_string()));
                assert_eq!(items[1], Value::String("i".to_string()));
                assert_eq!(items[2], Value::String("!".to_string()));
            }
            _ => panic!("Expected List"),
        }
        // Empty string → empty list
        let empty = call_string_method("string.chars", &[Value::String(String::new())]).unwrap();
        assert!(matches!(empty, Value::List(v) if v.is_empty()));
    }

    // --- New Number Method Tests ---

    #[test]
    fn test_number_abs() {
        assert_eq!(
            call_number_method("number.abs", &[Value::Number(-5.0)]).unwrap(),
            Value::Number(5.0)
        );
        assert_eq!(
            call_number_method("number.abs", &[Value::Number(5.0)]).unwrap(),
            Value::Number(5.0)
        );
    }

    #[test]
    fn test_number_floor() {
        assert_eq!(
            call_number_method("number.floor", &[Value::Number(3.7)]).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            call_number_method("number.floor", &[Value::Number(-3.2)]).unwrap(),
            Value::Number(-4.0)
        );
    }

    #[test]
    fn test_number_ceil() {
        assert_eq!(
            call_number_method("number.ceil", &[Value::Number(3.2)]).unwrap(),
            Value::Number(4.0)
        );
        assert_eq!(
            call_number_method("number.ceil", &[Value::Number(-3.7)]).unwrap(),
            Value::Number(-3.0)
        );
    }

    #[test]
    fn test_number_round() {
        assert_eq!(
            call_number_method("number.round", &[Value::Number(3.5)]).unwrap(),
            Value::Number(4.0)
        );
        assert_eq!(
            call_number_method("number.round", &[Value::Number(3.4)]).unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn test_number_sqrt() {
        assert_eq!(
            call_number_method("number.sqrt", &[Value::Number(9.0)]).unwrap(),
            Value::Number(3.0)
        );
        assert!(call_number_method("number.sqrt", &[Value::Number(-1.0)]).is_err());
    }

    #[test]
    fn test_number_is_infinite() {
        assert_eq!(
            call_number_method("number.is_infinite", &[Value::Number(f64::INFINITY)]).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            call_number_method("number.is_infinite", &[Value::Number(42.0)]).unwrap(),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_number_is_finite() {
        assert_eq!(
            call_number_method("number.is_finite", &[Value::Number(42.0)]).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            call_number_method("number.is_finite", &[Value::Number(f64::INFINITY)]).unwrap(),
            Value::Boolean(false)
        );
    }

    // --- New List Method Tests ---

    #[test]
    fn test_list_set() {
        let args = vec![
            Value::List(vec![
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
            ]),
            Value::Number(1.0),
            Value::Number(99.0),
        ];
        let result = call_list_method("list.set", &args).unwrap();
        match result {
            Value::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[1], Value::Number(99.0));
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_list_set_out_of_bounds() {
        let args = vec![
            Value::List(vec![Value::Number(1.0)]),
            Value::Number(5.0),
            Value::Number(99.0),
        ];
        assert!(call_list_method("list.set", &args).is_err());
    }

    #[test]
    fn test_list_first() {
        let args = vec![Value::List(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
        ])];
        assert_eq!(
            call_list_method("list.first", &args).unwrap(),
            Value::String("a".to_string())
        );
    }

    #[test]
    fn test_list_first_empty() {
        let args = vec![Value::List(vec![])];
        assert!(call_list_method("list.first", &args).is_err());
    }

    #[test]
    fn test_list_last() {
        let args = vec![Value::List(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
        ])];
        assert_eq!(
            call_list_method("list.last", &args).unwrap(),
            Value::String("b".to_string())
        );
    }

    #[test]
    fn test_list_reverse() {
        let args = vec![Value::List(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ])];
        let result = call_list_method("list.reverse", &args).unwrap();
        match result {
            Value::List(items) => {
                assert_eq!(items[0], Value::Number(3.0));
                assert_eq!(items[2], Value::Number(1.0));
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_list_is_empty() {
        assert_eq!(
            call_list_method("list.is_empty", &[Value::List(vec![])]).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            call_list_method("list.is_empty", &[Value::List(vec![Value::Number(1.0)])]).unwrap(),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_list_contains_not_found() {
        let args = vec![
            Value::List(vec![Value::Number(1.0), Value::Number(2.0)]),
            Value::Number(99.0),
        ];
        assert_eq!(
            call_list_method("list.contains", &args).unwrap(),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_list_get_out_of_bounds() {
        let args = vec![Value::List(vec![Value::Number(1.0)]), Value::Number(5.0)];
        assert!(call_list_method("list.get", &args).is_err());
    }

    #[test]
    fn test_unknown_list_method() {
        let args = vec![Value::List(vec![])];
        assert!(call_list_method("list.unknown", &args).is_err());
    }

    // --- New Map Method Tests ---

    #[test]
    fn test_map_remove() {
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), Value::Number(1.0));
        map.insert("b".to_string(), Value::Number(2.0));
        let args = vec![Value::Map(map), Value::String("a".to_string())];
        let result = call_map_method("map.remove", &args).unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(m.len(), 1);
                assert!(!m.contains_key("a"));
                assert!(m.contains_key("b"));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_map_merge() {
        let mut m1 = std::collections::HashMap::new();
        m1.insert("a".to_string(), Value::Number(1.0));
        let mut m2 = std::collections::HashMap::new();
        m2.insert("b".to_string(), Value::Number(2.0));
        let args = vec![Value::Map(m1), Value::Map(m2)];
        let result = call_map_method("map.merge", &args).unwrap();
        match result {
            Value::Map(m) => {
                assert_eq!(m.len(), 2);
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_map_len() {
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), Value::Number(1.0));
        map.insert("b".to_string(), Value::Number(2.0));
        assert_eq!(
            call_map_method("map.len", &[Value::Map(map)]).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn test_map_is_empty() {
        assert_eq!(
            call_map_method(
                "map.is_empty",
                &[Value::Map(std::collections::HashMap::new())]
            )
            .unwrap(),
            Value::Boolean(true)
        );
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), Value::Number(1.0));
        assert_eq!(
            call_map_method("map.is_empty", &[Value::Map(map)]).unwrap(),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_map_values() {
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), Value::Number(1.0));
        let result = call_map_method("map.values", &[Value::Map(map)]).unwrap();
        match result {
            Value::List(values) => assert_eq!(values.len(), 1),
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_map_has_key_not_found() {
        let map = std::collections::HashMap::new();
        let args = vec![Value::Map(map), Value::String("missing".to_string())];
        assert_eq!(
            call_map_method("map.has_key", &args).unwrap(),
            Value::Boolean(false)
        );
    }

    #[test]
    fn test_map_get_default() {
        let map = std::collections::HashMap::new();
        let args = vec![
            Value::Map(map),
            Value::String("missing".to_string()),
            Value::String("default".to_string()),
        ];
        assert_eq!(
            call_map_method("map.get", &args).unwrap(),
            Value::String("default".to_string())
        );
    }

    #[test]
    fn test_unknown_map_method() {
        let args = vec![Value::Map(std::collections::HashMap::new())];
        assert!(call_map_method("map.unknown", &args).is_err());
    }

    #[test]
    fn test_map_column_key_value() {
        let mut map = std::collections::HashMap::new();
        map.insert("b".to_string(), Value::String("2".to_string()));
        map.insert("a".to_string(), Value::String("100".to_string()));
        let s = call_map_method("map.column", &[Value::Map(map)])
            .unwrap()
            .as_string()
            .unwrap()
            .clone();
        assert_eq!(s, "a  100\nb  2  ");
    }

    #[test]
    fn test_map_column_tabular() {
        let mut map = std::collections::HashMap::new();
        map.insert(
            "name".to_string(),
            Value::List(vec![
                Value::String("Ann".to_string()),
                Value::String("Bob".to_string()),
            ]),
        );
        map.insert(
            "id".to_string(),
            Value::List(vec![Value::Number(1.0), Value::Number(10.0)]),
        );
        let s = call_map_method("map.column", &[Value::Map(map)])
            .unwrap()
            .as_string()
            .unwrap()
            .clone();
        assert_eq!(s, "id  name\n1   Ann \n10  Bob ");
    }

    #[test]
    fn test_map_column_mixed_lists_errors() {
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), Value::List(vec![Value::Number(1.0)]));
        map.insert("b".to_string(), Value::String("x".to_string()));
        assert!(call_map_method("map.column", &[Value::Map(map)]).is_err());
    }
}
