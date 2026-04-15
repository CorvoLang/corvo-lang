//! Rich diagnostic reporting for Corvo errors, inspired by `cargo clippy`.
//!
//! This module provides formatted error output with source context, visual
//! pointers, and "did you mean?" suggestions for unknown function calls.

use crate::ast::{Expr, Program, Stmt};
use crate::error::CorvoError;
use crate::span::Span;
use miette::{Diagnostic, GraphicalReportHandler, NamedSource, SourceSpan};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Internal diagnostic type that miette can render
// ---------------------------------------------------------------------------

#[derive(Debug, Error, Diagnostic)]
#[error("{_message}")]
struct CorvoReport {
    _message: String,
    #[source_code]
    _src: NamedSource,
    #[label("{_label}")]
    _span: Option<SourceSpan>,
    _label: String,
    #[help]
    _help: Option<String>,
}

// ---------------------------------------------------------------------------
// Public print helpers
// ---------------------------------------------------------------------------

/// Print a rich diagnostic for a `CorvoError`.
///
/// When the error contains span information and a source string is provided,
/// the output shows the offending line with a visual pointer – similar to
/// `rustc` or `cargo clippy`. When no span is available the output falls
/// back to a plain error message.
///
/// # Arguments
/// * `error`    – The error to display.
/// * `source`   – The full source text of the file being analysed.
/// * `filename` – The file name shown in the diagnostic header.
#[allow(unused_assignments)]
pub fn print_error(error: &CorvoError, source: &str, filename: &str) {
    let message = format!("{}", error);
    let help = get_help(error);
    let label = error.kind_label().to_string();

    let miette_span = error.span().map(|s| span_to_miette(&s, source));

    let report = CorvoReport {
        _message: message,
        _src: NamedSource::new(filename, source.to_string()),
        _span: miette_span,
        _label: label,
        _help: help,
    };

    let mut output = String::new();
    if GraphicalReportHandler::new()
        .render_report(&mut output, &report)
        .is_ok()
    {
        eprint!("{}", output);
    } else {
        // Fallback to plain text if rendering fails
        eprintln!("error: {}", report._message);
        if let Some(h) = &report._help {
            eprintln!("  help: {}", h);
        }
    }
}

/// Print a rich diagnostic when no source is available (e.g. eval mode).
pub fn print_error_no_source(error: &CorvoError) {
    let span_str = error
        .span()
        .map(|s| format!(" at line {}, column {}", s.start.line, s.start.column))
        .unwrap_or_default();

    eprintln!("error[{}]: {}{}", error.kind_label(), error, span_str);

    if let Some(help) = get_help(error) {
        eprintln!("  help: {}", help);
    }
}

// ---------------------------------------------------------------------------
// Lint pass – static analysis of a parsed program
// ---------------------------------------------------------------------------

/// A single diagnostic produced by the static lint analysis.
#[derive(Debug)]
pub struct LintDiagnostic {
    /// Human-readable description of the issue.
    pub message: String,
    /// Optional guidance shown below the error.
    pub help: Option<String>,
    /// Severity of the issue.
    pub severity: LintSeverity,
}

/// Severity level for a lint diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warning,
}

impl std::fmt::Display for LintSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
        }
    }
}

/// Walk a parsed `Program` and return all lint diagnostics found.
///
/// Currently checks:
/// * Every `Expr::FunctionCall` against the list of known built-in functions.
pub fn lint_program(program: &Program) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();
    for stmt in &program.statements {
        lint_stmt(stmt, &mut diags);
    }
    diags
}

fn lint_stmt(stmt: &Stmt, out: &mut Vec<LintDiagnostic>) {
    match stmt {
        Stmt::ExprStmt { expr } => lint_expr(expr, out),
        Stmt::VarSet { value, .. } => lint_expr(value, out),
        Stmt::StaticSet { value, .. } => lint_expr(value, out),
        Stmt::PrepBlock { body } => {
            for s in body {
                lint_stmt(s, out);
            }
        }
        Stmt::TryBlock { body, fallbacks } => {
            for s in body {
                lint_stmt(s, out);
            }
            for fb in fallbacks {
                for s in &fb.body {
                    lint_stmt(s, out);
                }
            }
        }
        Stmt::Loop { body } => {
            for s in body {
                lint_stmt(s, out);
            }
        }
        Stmt::Browse { iterable, body, .. } => {
            lint_expr(iterable, out);
            for s in body {
                lint_stmt(s, out);
            }
        }
        Stmt::Assert { args, .. } => {
            for a in args {
                lint_expr(a, out);
            }
        }
        Stmt::DontPanic { body } => {
            for s in body {
                lint_stmt(s, out);
            }
        }
        Stmt::AsyncBrowse {
            list, shared_vars, ..
        } => {
            lint_expr(list, out);
            if !shared_vars.is_empty() {
                out.push(LintDiagnostic {
                    message: "async_browse: shared variables are serialized at write-back"
                        .to_string(),
                    help: Some(
                        "Write-back order is non-deterministic between threads. \
                         Ensure the result accumulation is order-independent."
                            .to_string(),
                    ),
                    severity: LintSeverity::Warning,
                });
            }
        }
        Stmt::Terminate => {}
        Stmt::VarIndexSet { index, value, .. } => {
            lint_expr(index, out);
            lint_expr(value, out);
        }
        Stmt::VarAddAssign { value, .. } => lint_expr(value, out),
        Stmt::VarSubAssign { value, .. } => lint_expr(value, out),
        Stmt::VarOrAssign { candidates, .. } => {
            for candidate in candidates {
                lint_expr(candidate, out);
            }
        }
    }
}

fn lint_expr(expr: &Expr, out: &mut Vec<LintDiagnostic>) {
    match expr {
        Expr::FunctionCall {
            name,
            args,
            named_args,
        } => {
            // Skip internal pseudo-functions
            if name != "__list__" && name != "__map__" && !is_known_function(name) {
                let help = suggest_function(name)
                    .map(|s| format!("Did you mean `{}`?", s))
                    .or_else(|| {
                        // Suggest the namespace if the prefix exists
                        let prefix = name.split('.').next().unwrap_or("");
                        if KNOWN_NAMESPACES.contains(&prefix) {
                            Some(format!(
                                "The `{}` namespace exists. \
                                 Check the documentation for available functions.",
                                prefix
                            ))
                        } else {
                            None
                        }
                    });

                out.push(LintDiagnostic {
                    message: format!("Unknown function: `{}`", name),
                    help,
                    severity: LintSeverity::Error,
                });
            }

            for a in args {
                lint_expr(a, out);
            }
            for a in named_args.values() {
                lint_expr(a, out);
            }
        }
        Expr::StringInterpolation { parts } => {
            for p in parts {
                lint_expr(p, out);
            }
        }
        Expr::IndexAccess { target, index } => {
            lint_expr(target, out);
            lint_expr(index, out);
        }
        Expr::SliceAccess { target, start, end } => {
            lint_expr(target, out);
            if let Some(s) = start {
                lint_expr(s, out);
            }
            if let Some(e) = end {
                lint_expr(e, out);
            }
        }
        Expr::Match { value, arms } => {
            lint_expr(value, out);
            for arm in arms {
                lint_expr(&arm.body, out);
            }
        }
        Expr::MethodCall {
            target,
            args,
            named_args,
            ..
        } => {
            lint_expr(target, out);
            for a in args {
                lint_expr(a, out);
            }
            for a in named_args.values() {
                lint_expr(a, out);
            }
        }
        Expr::ProcedureLiteral { body, .. } => {
            for stmt in body {
                lint_stmt(stmt, out);
            }
        }
        Expr::Unary { operand, .. } => {
            lint_expr(operand, out);
        }
        Expr::Binary { left, right, .. } => {
            lint_expr(left, out);
            lint_expr(right, out);
        }
        Expr::Literal { .. }
        | Expr::VarGet { .. }
        | Expr::StaticGet { .. }
        | Expr::SharedArg { .. } => {}
    }
}

// ---------------------------------------------------------------------------
// "Did you mean?" helpers
// ---------------------------------------------------------------------------

/// All namespaces recognised by the standard library.
const KNOWN_NAMESPACES: &[&str] = &[
    "sys",
    "os",
    "math",
    "fs",
    "http",
    "dns",
    "crypto",
    "json",
    "yaml",
    "hcl",
    "csv",
    "xml",
    "env",
    "args",
    "time",
    "llm",
    "notifications",
    "re",
    "string",
    "number",
    "list",
    "map",
    "var",
    "static",
];

/// All functions recognised by the standard library and type system.
pub const KNOWN_FUNCTIONS: &[&str] = &[
    // sys
    "sys.echo",
    "sys.print",
    "sys.eprint",
    "sys.read_line",
    "sys.sleep",
    "sys.panic",
    "sys.exit",
    "sys.exec",
    // os
    "os.get_env",
    "os.set_env",
    "os.exec",
    "os.info",
    "os.argv",
    "os.getcwd",
    "os.uptime",
    "os.load_average",
    "os.user_count",
    "os.user_id",
    "os.group_id",
    // math
    "math.add",
    "math.sub",
    "math.mul",
    "math.div",
    "math.mod",
    "math.max",
    "math.human_bytes",
    // fs
    "fs.read",
    "fs.write",
    "fs.append",
    "fs.delete",
    "fs.exists",
    "fs.mkdir",
    "fs.list_dir",
    "fs.copy",
    "fs.move",
    "fs.stat",
    "fs.read_link",
    "fs.read_dir_meta",
    "fs.read_meta",
    "fs.path_parent",
    "fs.path_relative",
    "fs.chmod",
    "fs.chown",
    "fs.selinux_context_get",
    "fs.selinux_context_set",
    // http
    "http.get",
    "http.post",
    "http.put",
    "http.delete",
    // net (TCP)
    "net.tcp_listen",
    "net.tcp_accept",
    "net.tcp_close_listener",
    "net.tcp_connect",
    "net.tcp_read",
    "net.tcp_write",
    "net.tcp_close",
    // dns
    "dns.resolve",
    "dns.lookup",
    // crypto
    "crypto.hash",
    "crypto.hash_file",
    "crypto.checksum",
    "crypto.crc32_file",
    "crypto.encrypt",
    "crypto.decrypt",
    "crypto.uuid",
    // json
    "json.parse",
    "json.stringify",
    // yaml
    "yaml.parse",
    "yaml.stringify",
    // hcl
    "hcl.parse",
    "hcl.stringify",
    // csv
    "csv.parse",
    // xml
    "xml.parse",
    // env
    "env.parse",
    // args
    "args.scan",
    "args.parse",
    // time
    "time.format_local",
    "time.format_utc",
    "time.unix_now",
    "time.parse_date",
    "time.boot_time",
    // template
    "template.render",
    "template.render_file",
    // llm
    "llm.model",
    "llm.prompt",
    "llm.embed",
    "llm.chat",
    // notifications
    "notifications.smtp",
    "notifications.slack",
    "notifications.telegram",
    "notifications.mattermost",
    "notifications.gitter",
    "notifications.messenger",
    "notifications.discord",
    "notifications.teams",
    "notifications.x",
    "notifications.os",
    "notifications.irc",
    // regex methods
    "re.match",
    "re.find",
    "re.find_all",
    "re.replace",
    "re.replace_all",
    "re.split",
    "re.new",
    // string methods
    "string.concat",
    "string.replace",
    "string.split",
    "string.trim",
    "string.trim_start",
    "string.trim_end",
    "string.contains",
    "string.starts_with",
    "string.ends_with",
    "string.to_lower",
    "string.to_upper",
    "string.len",
    "string.reverse",
    "string.is_empty",
    "string.pad_start",
    "string.pad_end",
    "string.fnmatch",
    "string.byte_slice",
    "string.substring",
    "string.index_of",
    "string.char_at",
    "string.repeat",
    "string.replace_first",
    "string.count",
    "string.chars",
    "string.base64_encode",
    "string.base64_decode",
    "string.base32_encode",
    "string.base32_decode",
    "string.base32hex_encode",
    "string.base32hex_decode",
    "string.hex_encode",
    "string.hex_decode",
    // number methods
    "number.to_string",
    "number.parse",
    "number.is_nan",
    "number.is_infinite",
    "number.is_finite",
    "number.abs",
    "number.floor",
    "number.ceil",
    "number.round",
    "number.sqrt",
    "number.pow",
    "number.min",
    "number.max",
    "number.clamp",
    // list methods
    "list.push",
    "list.pop",
    "list.get",
    "list.set",
    "list.len",
    "list.first",
    "list.last",
    "list.is_empty",
    "list.contains",
    "list.delete",
    "list.filter",
    "list.map",
    "list.reduce",
    "list.find",
    "list.sort",
    "list.sort_version",
    "list.sort_maps_by_key",
    "list.columnate",
    "list.reverse",
    "list.flatten",
    "list.unique",
    "list.join",
    "list.slice",
    "list.new",
    // map methods
    "map.get",
    "map.set",
    "map.has",
    "map.has_key",
    "map.delete",
    "map.remove",
    "map.keys",
    "map.values",
    "map.entries",
    "map.len",
    "map.is_empty",
    "map.merge",
    "map.new",
    "map.column",
    // var / static (handled by parser but may appear as function calls in some paths)
    "var.get",
    "var.set",
    "static.get",
    "static.set",
];

fn is_known_function(name: &str) -> bool {
    KNOWN_FUNCTIONS.contains(&name)
}

/// Suggest the closest known function name using Levenshtein distance.
/// Returns `None` if no function is close enough to be a plausible suggestion.
pub fn suggest_function(name: &str) -> Option<String> {
    let threshold = (name.len() / 3 + 1).min(4);
    KNOWN_FUNCTIONS
        .iter()
        .filter_map(|&known| {
            let dist = levenshtein(name, known);
            if dist <= threshold {
                Some((dist, known))
            } else {
                None
            }
        })
        .min_by_key(|(d, _)| *d)
        .map(|(_, s)| s.to_string())
}

/// Compute the Levenshtein edit distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();

    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(n + 1) {
        *cell = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[m][n]
}

// ---------------------------------------------------------------------------
// Help text generation
// ---------------------------------------------------------------------------

fn get_help(error: &CorvoError) -> Option<String> {
    match error {
        CorvoError::UnknownFunction { name, .. } => suggest_function(name)
            .map(|s| format!("Did you mean `{}`?", s))
            .or_else(|| {
                let prefix = name.split('.').next().unwrap_or("");
                if KNOWN_NAMESPACES.contains(&prefix) {
                    Some(format!(
                        "The `{}` namespace exists but `{}` is not a known function in it.",
                        prefix, name
                    ))
                } else {
                    None
                }
            }),

        CorvoError::VariableNotFound { name, .. } => Some(format!(
            "Set the variable first with: var.set(\"{}\", <value>)",
            name
        )),

        CorvoError::StaticNotFound { name, .. } => Some(format!(
            "Define the static inside a `prep` block: prep {{ static.set(\"{}\", <value>) }}",
            name
        )),

        CorvoError::StaticModification { name, .. } => Some(format!(
            "`{}` is a static value and cannot be modified after the prep block.",
            name
        )),

        CorvoError::Parsing { .. } => {
            Some("Check the Corvo language documentation for correct syntax.".to_string())
        }

        CorvoError::Lexing { .. } => {
            Some("This character or token is not valid in Corvo source code.".to_string())
        }

        CorvoError::DivisionByZero { .. } => {
            Some("Ensure the divisor is non-zero before calling `math.div`.".to_string())
        }

        CorvoError::Type { .. } => Some(
            "Check that the values you are passing have the expected types. \
             Use `string.*`, `number.*`, `list.*`, or `map.*` methods to convert them."
                .to_string(),
        ),

        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Internal: convert a Corvo `Span` to a `miette::SourceSpan`
// ---------------------------------------------------------------------------

fn span_to_miette(span: &Span, source: &str) -> SourceSpan {
    let start = span.start.offset.min(source.len());
    let end = span.end.offset.min(source.len());
    let len = end.saturating_sub(start).max(1);
    SourceSpan::new(start.into(), len.into())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_same() {
        assert_eq!(levenshtein("sys.echo", "sys.echo"), 0);
    }

    #[test]
    fn test_levenshtein_single_typo() {
        // "sys.ecoh" vs "sys.echo" – transposition = distance 2
        assert!(levenshtein("sys.ecoh", "sys.echo") <= 2);
    }

    #[test]
    fn test_suggest_function_typo() {
        let suggestion = suggest_function("sys.prin");
        // Should suggest sys.panic or sys.echo – something close
        assert!(suggestion.is_some());
    }

    #[test]
    fn test_suggest_function_exact() {
        // Exact match → distance 0 → always suggested
        assert_eq!(suggest_function("sys.echo"), Some("sys.echo".to_string()));
    }

    #[test]
    fn test_suggest_function_no_match() {
        // Completely unrelated name should return None
        let suggestion = suggest_function("zzzzzzzzzzzzzzzzzzzzz");
        assert!(suggestion.is_none());
    }

    #[test]
    fn test_is_known_function() {
        assert!(is_known_function("sys.echo"));
        assert!(is_known_function("math.add"));
        assert!(!is_known_function("sys.unknown_func"));
    }

    #[test]
    fn test_span_to_miette() {
        use crate::span::Position;
        let source = "hello world";
        let span = Span::new(Position::new(1, 1, 0), Position::new(1, 6, 5));
        let ms = span_to_miette(&span, source);
        assert_eq!(ms.offset(), 0);
        assert_eq!(ms.len(), 5);
    }

    #[test]
    fn test_lint_program_unknown_function() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let source = r#"sys.ecoh("hello")"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse().unwrap();
        let diags = lint_program(&program);

        assert!(!diags.is_empty());
        assert!(diags[0].message.contains("sys.ecoh"));
        assert_eq!(diags[0].severity, LintSeverity::Error);
    }

    #[test]
    fn test_lint_program_known_function() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let source = r#"sys.echo("hello")"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse().unwrap();
        let diags = lint_program(&program);

        assert!(diags.is_empty());
    }
}
