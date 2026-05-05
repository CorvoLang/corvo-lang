//! Command-line argument parsing for Corvo scripts.
//!
//! ## Functions
//!
//! - **`args.parse(argv, config?)`** — generic configurable parser (GNU coreutils,
//!   dnsutils dig-style `+` flags, usbutils colon-compound values, …).
//! - **`args.scan(argv)`** — simple POSIX-ish scan; delegates to `args.parse` with
//!   no config (backward-compatible with the previous `args.scan`).
//!
//! ## `args.parse` config map keys (all optional)
//!
//! | key | type | description |
//! |-----|------|-------------|
//! | `"aliases"` | map | raw-key → semantic output key |
//! | `"short_values"` | list | short chars that consume a glued tail or next token |
//! | `"long_values"` | list | long flag names (no `--`, normalized) that require a value |
//! | `"long_optional_values"` | list | long flags whose value is attached via `=` only |
//! | `"accumulate"` | list | output keys where repeated values build a list |
//! | `"plus_flags"` | bool | enable dig-style `+flag` / `+noflag` / `+key=val` → `"plus"` map |
//! | `"at_tokens"` | bool | collect `@server` tokens into `"at_servers"` list |
//! | `"permute"` | bool (default true) | interleave options and operands (GNU) vs stop at first positional (POSIX) |

use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn strings_from_list(arg: &Value, ctx: &str) -> CorvoResult<Vec<String>> {
    let list = arg.as_list().ok_or_else(|| {
        CorvoError::invalid_argument(format!("{ctx}: expected a list of strings"))
    })?;
    let mut out = Vec::with_capacity(list.len());
    for v in list {
        let s = v.as_string().ok_or_else(|| {
            CorvoError::invalid_argument(format!("{ctx}: list must contain only strings"))
        })?;
        out.push(s.clone());
    }
    Ok(out)
}

/// Normalise a long-option body: strip leading `--`, replace `-` with `_`.
fn normalise_long(body: &str) -> String {
    body.replace('-', "_")
}

/// Resolve an alias: look up the (already-normalised) key in the aliases map;
/// fall back to the key itself.
fn resolve_alias<'a>(key: &'a str, aliases: &'a HashMap<String, String>) -> &'a str {
    aliases.get(key).map(|s| s.as_str()).unwrap_or(key)
}

/// Insert or append to the options map, honouring `accumulate`.
fn store(opts: &mut HashMap<String, Value>, key: &str, val: Value, accumulate: &[String]) {
    if accumulate.iter().any(|k| k == key) {
        match opts.get_mut(key) {
            Some(Value::List(list)) => {
                list.push(val);
                return;
            }
            Some(existing) => {
                let prev = existing.clone();
                *existing = Value::List(vec![prev, val]);
                return;
            }
            None => {
                opts.insert(key.to_string(), Value::List(vec![val]));
                return;
            }
        }
    }
    opts.insert(key.to_string(), val);
}

// ---------------------------------------------------------------------------
// Config extraction helpers
// ---------------------------------------------------------------------------

struct ParseConfig {
    aliases: HashMap<String, String>,
    short_values: Vec<String>,
    long_values: Vec<String>,
    long_optional_values: Vec<String>,
    accumulate: Vec<String>,
    plus_flags: bool,
    at_tokens: bool,
    permute: bool,
}

impl ParseConfig {
    fn from_value(cfg: Option<&Value>) -> CorvoResult<Self> {
        let map = match cfg {
            None | Some(Value::Null) => return Ok(Self::default()),
            Some(v) => v
                .as_map()
                .ok_or_else(|| CorvoError::invalid_argument("args.parse: config must be a map"))?,
        };

        let aliases = match map.get("aliases") {
            Some(Value::Map(m)) => m
                .iter()
                .filter_map(|(k, v)| v.as_string().map(|s| (k.clone(), s.clone())))
                .collect(),
            _ => HashMap::new(),
        };

        let short_values = match map.get("short_values") {
            Some(v) => strings_from_list(v, "args.parse short_values")?,
            None => Vec::new(),
        };
        let long_values = match map.get("long_values") {
            Some(v) => strings_from_list(v, "args.parse long_values")?,
            None => Vec::new(),
        };
        let long_optional_values = match map.get("long_optional_values") {
            Some(v) => strings_from_list(v, "args.parse long_optional_values")?,
            None => Vec::new(),
        };
        let accumulate = match map.get("accumulate") {
            Some(v) => strings_from_list(v, "args.parse accumulate")?,
            None => Vec::new(),
        };

        let plus_flags = map
            .get("plus_flags")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let at_tokens = map
            .get("at_tokens")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let permute = map.get("permute").and_then(|v| v.as_bool()).unwrap_or(true);

        Ok(Self {
            aliases,
            short_values,
            long_values,
            long_optional_values,
            accumulate,
            plus_flags,
            at_tokens,
            permute,
        })
    }
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            aliases: HashMap::new(),
            short_values: Vec::new(),
            long_values: Vec::new(),
            long_optional_values: Vec::new(),
            accumulate: Vec::new(),
            plus_flags: false,
            at_tokens: false,
            permute: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Core parser
// ---------------------------------------------------------------------------

struct ParseResult {
    opts: HashMap<String, Value>,
    pos: Vec<String>,
    plus: Option<HashMap<String, Value>>,
    at_servers: Option<Vec<String>>,
}

fn do_parse(tokens: &[String], cfg: &ParseConfig) -> ParseResult {
    let mut opts: HashMap<String, Value> = HashMap::new();
    let mut pos: Vec<String> = Vec::new();
    let mut plus_map: HashMap<String, Value> = HashMap::new();
    let mut at_servers: Vec<String> = Vec::new();

    let mut i = 0usize;

    while i < tokens.len() {
        let tok = &tokens[i];

        // --- end-of-options sentinel: consume everything after `--` as positional ---
        if tok == "--" {
            i += 1;
            for t in tokens.iter().skip(i) {
                pos.push(t.clone());
            }
            break;
        }

        // POSIX mode: stop scanning options after first positional
        if !cfg.permute && !pos.is_empty() {
            pos.push(tok.clone());
            i += 1;
            continue;
        }

        // --- lone `-` is a positional (stdin marker) ---
        if tok == "-" {
            pos.push(tok.clone());
            i += 1;
            continue;
        }

        // --- @server token (dig-style) ---
        if cfg.at_tokens && tok.starts_with('@') {
            at_servers.push(tok[1..].to_string());
            i += 1;
            continue;
        }

        // --- plus-option token (dig-style): +flag, +noflag, +key=val ---
        if cfg.plus_flags && tok.starts_with('+') && tok.len() > 1 {
            let body = &tok[1..];
            if let Some(eq) = body.find('=') {
                let key = normalise_long(&body[..eq]);
                let val = body[eq + 1..].to_string();
                plus_map.insert(key, Value::String(val));
            } else if let Some(rest) = body.strip_prefix("no") {
                let key = normalise_long(rest);
                plus_map.insert(key, Value::Boolean(false));
            } else {
                let key = normalise_long(body);
                plus_map.insert(key, Value::Boolean(true));
            }
            i += 1;
            continue;
        }

        // --- long option: --name, --name=val, --name val ---
        if tok.starts_with("--") && tok.len() > 2 {
            let body = &tok[2..];
            let (raw_name, eq_val): (String, Option<String>) = if let Some(p) = body.find('=') {
                (body[..p].to_string(), Some(body[p + 1..].to_string()))
            } else {
                (body.to_string(), None)
            };
            let norm = normalise_long(&raw_name);
            i += 1;

            // optional-value long option: value only via `=`
            if cfg
                .long_optional_values
                .iter()
                .any(|s| normalise_long(s) == norm)
            {
                let val = eq_val.unwrap_or_else(|| "always".to_string());
                let key = resolve_alias(&norm, &cfg.aliases);
                store(&mut opts, key, Value::String(val), &cfg.accumulate);
                continue;
            }

            // required-value long option
            if cfg.long_values.iter().any(|s| normalise_long(s) == norm) {
                let val = if let Some(v) = eq_val {
                    v
                } else if i < tokens.len() && !tokens[i].starts_with('-') {
                    let v = tokens[i].clone();
                    i += 1;
                    v
                } else {
                    String::new()
                };
                if !val.is_empty() {
                    let key = resolve_alias(&norm, &cfg.aliases);
                    store(&mut opts, key, Value::String(val), &cfg.accumulate);
                }
                continue;
            }

            // boolean (or inline =value from unknown option)
            let key = resolve_alias(&norm, &cfg.aliases);
            if let Some(v) = eq_val {
                store(&mut opts, key, Value::String(v), &cfg.accumulate);
            } else {
                store(&mut opts, key, Value::Boolean(true), &cfg.accumulate);
            }
            continue;
        }

        // --- short option cluster: -lah, -w80, -w 80 ---
        if tok.starts_with('-') && tok.len() > 1 {
            let chars: Vec<char> = tok.chars().skip(1).collect();
            let mut k = 0usize;
            let start_i = i;
            i += 1; // default advance; may be overridden if we consume the next token

            while k < chars.len() {
                let c = chars[k];
                let ch = c.to_string();

                if cfg.short_values.iter().any(|s| s == &ch) {
                    // tail of the cluster is the value, or next argv token
                    let tail: String = chars.iter().skip(k + 1).collect();
                    let val = if !tail.is_empty() {
                        tail
                    } else if start_i + 1 < tokens.len() {
                        let v = tokens[start_i + 1].clone();
                        i = start_i + 2; // consumed next token
                        v
                    } else {
                        String::new()
                    };
                    if !val.is_empty() {
                        let key = resolve_alias(&ch, &cfg.aliases);
                        store(&mut opts, key, Value::String(val), &cfg.accumulate);
                    }
                    break; // value-bearing short flag ends the cluster
                }

                let key = resolve_alias(&ch, &cfg.aliases);
                store(&mut opts, key, Value::Boolean(true), &cfg.accumulate);
                k += 1;
            }
            continue;
        }

        // --- positional ---
        pos.push(tok.clone());
        if !cfg.permute {
            // POSIX mode: stop scanning options
            for t in tokens.iter().skip(i + 1) {
                pos.push(t.clone());
            }
            break;
        }
        i += 1;
    }

    ParseResult {
        opts,
        pos,
        plus: if cfg.plus_flags { Some(plus_map) } else { None },
        at_servers: if cfg.at_tokens {
            Some(at_servers)
        } else {
            None
        },
    }
}

// ---------------------------------------------------------------------------
// Public Corvo functions
// ---------------------------------------------------------------------------

/// Generic configurable argv parser.
/// Signature: `args.parse(argv: list[string], config?: map) -> map`
pub fn parse(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let tokens = if args.is_empty() {
        Vec::new()
    } else {
        strings_from_list(&args[0], "args.parse")?
    };

    let cfg = ParseConfig::from_value(args.get(1))?;
    let res = do_parse(&tokens, &cfg);

    let mut result: HashMap<String, Value> = HashMap::new();
    result.insert(
        "positional".to_string(),
        Value::List(res.pos.into_iter().map(Value::String).collect()),
    );
    result.insert("options".to_string(), Value::Map(res.opts));
    if let Some(pm) = res.plus {
        result.insert("plus".to_string(), Value::Map(pm));
    }
    if let Some(at) = res.at_servers {
        result.insert(
            "at_servers".to_string(),
            Value::List(at.into_iter().map(Value::String).collect()),
        );
    }
    Ok(Value::Map(result))
}

/// Simple POSIX-ish scan — delegates to `args.parse` with no config.
/// Kept for backward compatibility with scripts using `args.scan`.
pub fn scan(args: &[Value], named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    parse(args, named_args)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_named() -> HashMap<String, Value> {
        HashMap::new()
    }

    fn argv(vals: &[&str]) -> Value {
        Value::List(
            vals.iter()
                .map(|s| Value::String((*s).to_string()))
                .collect(),
        )
    }

    fn make_map(pairs: &[(&str, &str)]) -> Value {
        let mut m: HashMap<String, Value> = HashMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), Value::String(v.to_string()));
        }
        Value::Map(m)
    }

    fn make_list_val(vals: &[&str]) -> Value {
        Value::List(vals.iter().map(|s| Value::String(s.to_string())).collect())
    }

    fn make_cfg(pairs: &[(&str, Value)]) -> Value {
        let mut m: HashMap<String, Value> = HashMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), v.clone());
        }
        Value::Map(m)
    }

    fn run_parse(argv_val: Value, cfg: Option<Value>) -> (Vec<String>, HashMap<String, Value>) {
        let args_slice: Vec<Value> = if let Some(c) = cfg {
            vec![argv_val, c]
        } else {
            vec![argv_val]
        };
        let m = parse(&args_slice, &empty_named()).unwrap();
        let map = m.as_map().unwrap();
        let pos = map
            .get("positional")
            .unwrap()
            .as_list()
            .unwrap()
            .iter()
            .map(|v| v.as_string().unwrap().clone())
            .collect();
        let opts = map.get("options").unwrap().as_map().unwrap().clone();
        (pos, opts)
    }

    // ── backward-compat: scan() behaves like no-config parse() ──────────────

    #[test]
    fn empty_argv() {
        let (pos, opts) = run_parse(argv(&[]), None);
        assert!(pos.is_empty());
        assert!(opts.is_empty());
    }

    #[test]
    fn positionals_only() {
        let (pos, opts) = run_parse(argv(&["a", "b"]), None);
        assert_eq!(pos, vec!["a", "b"]);
        assert!(opts.is_empty());
    }

    #[test]
    fn double_dash_rest_positional() {
        let (pos, opts) = run_parse(argv(&["--foo", "--", "-x", "y"]), None);
        assert_eq!(pos, vec!["-x", "y"]);
        assert_eq!(opts.get("foo"), Some(&Value::Boolean(true)));
    }

    #[test]
    fn long_equals() {
        let (_pos, opts) = run_parse(argv(&["--out=file.txt"]), None);
        assert_eq!(
            opts.get("out"),
            Some(&Value::String("file.txt".to_string()))
        );
    }

    #[test]
    fn long_empty_equals() {
        let (_pos, opts) = run_parse(argv(&["--tag="]), None);
        assert_eq!(opts.get("tag"), Some(&Value::String("".to_string())));
    }

    #[test]
    fn long_bool_without_config() {
        // Zero-config: long options are boolean; the next token becomes positional.
        let (pos, opts) = run_parse(argv(&["--out", "path"]), None);
        assert_eq!(pos, vec!["path".to_string()]);
        assert_eq!(opts.get("out"), Some(&Value::Boolean(true)));
    }

    #[test]
    fn long_takes_next_with_long_values() {
        // With long_values config: --out consumes the next non-flag token as its value.
        let cfg = make_cfg(&[(
            "long_values",
            Value::List(vec![Value::String("out".to_string())]),
        )]);
        let (pos, opts) = run_parse(argv(&["--out", "path"]), Some(cfg));
        assert!(pos.is_empty());
        assert_eq!(opts.get("out"), Some(&Value::String("path".to_string())));
    }

    #[test]
    fn long_bool_when_next_is_flag() {
        let (_pos, opts) = run_parse(argv(&["--verbose", "--other"]), None);
        assert_eq!(opts.get("verbose"), Some(&Value::Boolean(true)));
        assert_eq!(opts.get("other"), Some(&Value::Boolean(true)));
    }

    #[test]
    fn short_cluster() {
        let (_pos, opts) = run_parse(argv(&["-abc"]), None);
        assert_eq!(opts.get("a"), Some(&Value::Boolean(true)));
        assert_eq!(opts.get("b"), Some(&Value::Boolean(true)));
        assert_eq!(opts.get("c"), Some(&Value::Boolean(true)));
    }

    #[test]
    fn lone_hyphen_positional() {
        let (pos, opts) = run_parse(argv(&["-"]), None);
        assert_eq!(pos, vec!["-"]);
        assert!(opts.is_empty());
    }

    #[test]
    fn duplicate_last_wins() {
        let (_pos, opts) = run_parse(argv(&["--x=1", "--x=2"]), None);
        assert_eq!(opts.get("x"), Some(&Value::String("2".to_string())));
    }

    #[test]
    fn missing_arg_errors() {
        assert!(parse(&[], &empty_named()).is_ok());
        let err = parse(&[Value::Number(1.0)], &empty_named()).unwrap_err();
        assert!(format!("{err}").contains("list"));
    }

    // ── short_values: glued tail and separate token ──────────────────────────

    #[test]
    fn short_value_glued() {
        let cfg = make_cfg(&[("short_values", make_list_val(&["w"]))]);
        let (_pos, opts) = run_parse(argv(&["-w80"]), Some(cfg));
        assert_eq!(opts.get("w"), Some(&Value::String("80".to_string())));
    }

    #[test]
    fn short_value_separate() {
        let cfg = make_cfg(&[("short_values", make_list_val(&["w"]))]);
        let (_pos, opts) = run_parse(argv(&["-w", "80"]), Some(cfg));
        assert_eq!(opts.get("w"), Some(&Value::String("80".to_string())));
    }

    #[test]
    fn short_value_in_cluster_stops() {
        // -lw80: l is bool, w80 means w=80 and cluster stops
        let cfg = make_cfg(&[("short_values", make_list_val(&["w"]))]);
        let (_pos, opts) = run_parse(argv(&["-lw80"]), Some(cfg));
        assert_eq!(opts.get("l"), Some(&Value::Boolean(true)));
        assert_eq!(opts.get("w"), Some(&Value::String("80".to_string())));
    }

    // ── long_values: required value ──────────────────────────────────────────

    #[test]
    fn long_value_eq() {
        let cfg = make_cfg(&[("long_values", make_list_val(&["sort"]))]);
        let (_pos, opts) = run_parse(argv(&["--sort=size"]), Some(cfg));
        assert_eq!(opts.get("sort"), Some(&Value::String("size".to_string())));
    }

    #[test]
    fn long_value_space() {
        let cfg = make_cfg(&[("long_values", make_list_val(&["sort"]))]);
        let (_pos, opts) = run_parse(argv(&["--sort", "time"]), Some(cfg));
        assert_eq!(opts.get("sort"), Some(&Value::String("time".to_string())));
    }

    // ── long_optional_values: default when no = ──────────────────────────────

    #[test]
    fn long_optional_bare() {
        let cfg = make_cfg(&[("long_optional_values", make_list_val(&["color"]))]);
        let (_pos, opts) = run_parse(argv(&["--color"]), Some(cfg));
        assert_eq!(
            opts.get("color"),
            Some(&Value::String("always".to_string()))
        );
    }

    #[test]
    fn long_optional_with_eq() {
        let cfg = make_cfg(&[("long_optional_values", make_list_val(&["color"]))]);
        let (_pos, opts) = run_parse(argv(&["--color=never"]), Some(cfg));
        assert_eq!(opts.get("color"), Some(&Value::String("never".to_string())));
    }

    // ── aliases ──────────────────────────────────────────────────────────────

    #[test]
    fn alias_short() {
        let cfg = make_cfg(&[(
            "aliases",
            make_map(&[("l", "long"), ("a", "all"), ("h", "human_readable")]),
        )]);
        let (_pos, opts) = run_parse(argv(&["-lah"]), Some(cfg));
        assert_eq!(opts.get("long"), Some(&Value::Boolean(true)));
        assert_eq!(opts.get("all"), Some(&Value::Boolean(true)));
        assert_eq!(opts.get("human_readable"), Some(&Value::Boolean(true)));
        assert!(!opts.contains_key("l"));
    }

    #[test]
    fn alias_long_normalised() {
        let cfg = make_cfg(&[("aliases", make_map(&[("almost_all", "almost_all")]))]);
        let (_pos, opts) = run_parse(argv(&["--almost-all"]), Some(cfg));
        assert_eq!(opts.get("almost_all"), Some(&Value::Boolean(true)));
    }

    // ── accumulate ───────────────────────────────────────────────────────────

    #[test]
    fn accumulate_short() {
        let cfg = make_cfg(&[
            ("short_values", make_list_val(&["I"])),
            ("accumulate", make_list_val(&["I"])),
        ]);
        let (_pos, opts) = run_parse(argv(&["-I", "*.o", "-I", "tmp"]), Some(cfg));
        let list = opts.get("I").unwrap().as_list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], Value::String("*.o".to_string()));
        assert_eq!(list[1], Value::String("tmp".to_string()));
    }

    #[test]
    fn accumulate_long() {
        let cfg = make_cfg(&[
            ("long_values", make_list_val(&["ignore"])),
            ("accumulate", make_list_val(&["ignore"])),
        ]);
        let (_pos, opts) = run_parse(argv(&["--ignore=*.o", "--ignore=tmp"]), Some(cfg));
        let list = opts.get("ignore").unwrap().as_list().unwrap();
        assert_eq!(list.len(), 2);
    }

    // ── plus_flags (dig-style) ───────────────────────────────────────────────

    #[test]
    fn plus_flag_bool() {
        let cfg = make_cfg(&[("plus_flags", Value::Boolean(true))]);
        let args_slice = vec![argv(&["+short"]), cfg];
        let m = parse(&args_slice, &empty_named()).unwrap();
        let plus = m
            .as_map()
            .unwrap()
            .get("plus")
            .unwrap()
            .as_map()
            .unwrap()
            .clone();
        assert_eq!(plus.get("short"), Some(&Value::Boolean(true)));
    }

    #[test]
    fn plus_flag_no_prefix() {
        let cfg = make_cfg(&[("plus_flags", Value::Boolean(true))]);
        let args_slice = vec![argv(&["+notcp"]), cfg];
        let m = parse(&args_slice, &empty_named()).unwrap();
        let plus = m
            .as_map()
            .unwrap()
            .get("plus")
            .unwrap()
            .as_map()
            .unwrap()
            .clone();
        assert_eq!(plus.get("tcp"), Some(&Value::Boolean(false)));
    }

    #[test]
    fn plus_flag_kv() {
        let cfg = make_cfg(&[("plus_flags", Value::Boolean(true))]);
        let args_slice = vec![argv(&["+time=3"]), cfg];
        let m = parse(&args_slice, &empty_named()).unwrap();
        let plus = m
            .as_map()
            .unwrap()
            .get("plus")
            .unwrap()
            .as_map()
            .unwrap()
            .clone();
        assert_eq!(plus.get("time"), Some(&Value::String("3".to_string())));
    }

    // ── at_tokens (dig-style) ────────────────────────────────────────────────

    #[test]
    fn at_token_collect() {
        let cfg = make_cfg(&[("at_tokens", Value::Boolean(true))]);
        let args_slice = vec![argv(&["@8.8.8.8", "example.com", "@1.1.1.1"]), cfg];
        let m = parse(&args_slice, &empty_named()).unwrap();
        let mmap = m.as_map().unwrap();
        let at = mmap.get("at_servers").unwrap().as_list().unwrap();
        assert_eq!(at.len(), 2);
        assert_eq!(at[0], Value::String("8.8.8.8".to_string()));
        assert_eq!(at[1], Value::String("1.1.1.1".to_string()));
        let pos = mmap.get("positional").unwrap().as_list().unwrap();
        assert_eq!(pos, &[Value::String("example.com".to_string())]);
    }

    // ── permute=false (POSIX) ────────────────────────────────────────────────

    #[test]
    fn permute_false_stops_at_first_positional() {
        let cfg = make_cfg(&[("permute", Value::Boolean(false))]);
        let (pos, opts) = run_parse(argv(&["-v", "file", "--other"]), Some(cfg));
        assert_eq!(opts.get("v"), Some(&Value::Boolean(true)));
        // --other comes after first positional: treated as positional in POSIX mode
        assert_eq!(pos, vec!["file", "--other"]);
    }

    // ── long option hyphen-to-underscore normalisation ───────────────────────

    #[test]
    fn long_hyphen_normalised_to_underscore() {
        let (_pos, opts) = run_parse(argv(&["--time-style=long-iso"]), None);
        assert_eq!(
            opts.get("time_style"),
            Some(&Value::String("long-iso".to_string()))
        );
    }

    // ── combined GNU ls spec (spot check) ───────────────────────────────────

    #[test]
    fn gnu_ls_cluster_with_width() {
        let mut alias_map: HashMap<String, Value> = HashMap::new();
        for (k, v) in [("l", "long"), ("a", "all"), ("h", "human_readable")] {
            alias_map.insert(k.to_string(), Value::String(v.to_string()));
        }
        let mut cfg_map: HashMap<String, Value> = HashMap::new();
        cfg_map.insert("aliases".to_string(), Value::Map(alias_map));
        cfg_map.insert(
            "short_values".to_string(),
            Value::List(vec![Value::String("w".to_string())]),
        );
        let (_pos, opts) = run_parse(
            argv(&["-lah", "-w", "80", "path"]),
            Some(Value::Map(cfg_map)),
        );
        assert_eq!(opts.get("long"), Some(&Value::Boolean(true)));
        assert_eq!(opts.get("all"), Some(&Value::Boolean(true)));
        assert_eq!(opts.get("human_readable"), Some(&Value::Boolean(true)));
        assert_eq!(opts.get("w"), Some(&Value::String("80".to_string())));
    }
}

#[macro_export]
macro_rules! args_scan {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("args.scan", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("args.scan", &[$($arg),*], &$kwargs, $state)
    };
}

#[macro_export]
macro_rules! args_parse {
    ($state:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("args.parse", &[$($arg),*], &std::collections::HashMap::new(), $state)
    };
    ($state:expr; kwargs: $kwargs:expr $(, $arg:expr)* $(,)?) => {
        $crate::standard_lib::call("args.parse", &[$($arg),*], &$kwargs, $state)
    };
}
