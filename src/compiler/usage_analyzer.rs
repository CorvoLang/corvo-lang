use crate::ast::{Expr, Program, Stmt};
use std::collections::HashSet;

/// Analyzes a parsed Corvo program to determine which standard library
/// functions, type methods, and special blocks are actually used.
///
/// This analysis drives the Oxide transpiler's feature-gated dependency
/// selection: only the Cargo features actually needed by the script are
/// included in the generated `Cargo.toml`.
#[derive(Debug, Clone)]
pub struct UsageAnalysis {
    /// Fully-qualified stdlib function names, e.g. `"sys.echo"`, `"fs.read"`.
    pub functions: HashSet<String>,
    /// Type method names, e.g. `"len"`, `"split"`, `"trim"`.
    pub type_methods: HashSet<String>,
    /// Namespace prefixes derived from `functions`, e.g. `"sys"`, `"fs"`.
    pub namespaces: HashSet<String>,
    /// Whether the script uses `http_listen { ... }` blocks.
    pub uses_http_listen: bool,
    /// Whether the script uses `amqp_consume { ... }` blocks.
    pub uses_amqp_consume: bool,
    /// Whether the script uses `async_browse { ... }` blocks.
    pub uses_async_browse: bool,
}

impl UsageAnalysis {
    /// Walk the entire program AST and collect usage information.
    pub fn from_program(program: &Program) -> Self {
        let mut analysis = Self {
            functions: HashSet::new(),
            type_methods: HashSet::new(),
            namespaces: HashSet::new(),
            uses_http_listen: false,
            uses_amqp_consume: false,
            uses_async_browse: false,
        };
        for stmt in &program.statements {
            analysis.walk_stmt(stmt);
        }
        // Derive namespaces from function names.
        for func in &analysis.functions {
            if let Some(ns) = func.split('.').next() {
                analysis.namespaces.insert(ns.to_string());
            }
        }
        analysis
    }

    /// Returns the set of Cargo feature flags required by this script.
    pub fn required_features(&self) -> Vec<String> {
        let mut features = Vec::new();

        // Map namespace → feature name
        let ns_map: &[(&str, &str)] = &[
            ("http", "stdlib-http"),
            ("crypto", "stdlib-crypto"),
            ("db", "stdlib-db"),
            ("amqp", "stdlib-amqp"),
            ("notifications", "stdlib-notifications"),
            ("llm", "stdlib-llm"),
            ("template", "stdlib-template"),
            ("dns", "stdlib-dns"),
            ("csv", "stdlib-csv"),
            ("xml", "stdlib-xml"),
            ("yaml", "stdlib-yaml"),
            ("env", "stdlib-env"),
            ("hcl", "stdlib-hcl"),
            ("net", "stdlib-net"),
        ];

        for (ns, feature) in ns_map {
            if self.namespaces.contains(*ns) {
                features.push(feature.to_string());
            }
        }

        // Special blocks that imply features
        if self.uses_amqp_consume {
            let f = "stdlib-amqp".to_string();
            if !features.contains(&f) {
                features.push(f);
            }
        }

        if self.uses_http_listen {
            let f = "stdlib-http".to_string();
            if !features.contains(&f) {
                features.push(f);
            }
        }

        // Check type methods that need specific features
        let crypto_methods = ["base64_encode", "base64_decode"];
        for m in &crypto_methods {
            if self.type_methods.contains(*m) {
                // base64 is always available (core dep), no feature needed
            }
        }

        features
    }

    fn walk_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarSet { value, .. } | Stmt::StaticSet { value, .. } => {
                self.walk_expr(value);
            }
            Stmt::VarIndexSet { index, value, .. } => {
                self.walk_expr(index);
                self.walk_expr(value);
            }
            Stmt::VarAddAssign { value, .. } | Stmt::VarSubAssign { value, .. } => {
                self.walk_expr(value);
            }
            Stmt::VarOrAssign { candidates, .. } => {
                for c in candidates {
                    self.walk_expr(c);
                }
            }
            Stmt::ExprStmt { expr } => {
                self.walk_expr(expr);
            }
            Stmt::TryBlock { body, fallbacks } => {
                for s in body {
                    self.walk_stmt(s);
                }
                for fb in fallbacks {
                    for s in &fb.body {
                        self.walk_stmt(s);
                    }
                }
            }
            Stmt::Loop { body } | Stmt::DontPanic { body } | Stmt::PrepBlock { body } => {
                for s in body {
                    self.walk_stmt(s);
                }
            }
            Stmt::Browse { iterable, body, .. } => {
                self.walk_expr(iterable);
                for s in body {
                    self.walk_stmt(s);
                }
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.walk_expr(condition);
                for s in then_branch {
                    self.walk_stmt(s);
                }
                for s in else_branch {
                    self.walk_stmt(s);
                }
            }
            Stmt::Assert { args, .. } => {
                for a in args {
                    self.walk_expr(a);
                }
            }
            Stmt::Terminate => {}
            Stmt::AsyncBrowse { list, .. } => {
                self.uses_async_browse = true;
                self.walk_expr(list);
            }
            Stmt::HttpListen { port, body, .. } => {
                self.uses_http_listen = true;
                self.walk_expr(port);
                for s in body {
                    self.walk_stmt(s);
                }
            }
            Stmt::AmqpConsume {
                connection,
                queue,
                body,
                ..
            } => {
                self.uses_amqp_consume = true;
                self.walk_expr(connection);
                self.walk_expr(queue);
                for s in body {
                    self.walk_stmt(s);
                }
            }
        }
    }

    fn walk_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::FunctionCall {
                name,
                args,
                named_args,
            } => {
                // Skip synthetic names like __list__, __map__
                if !name.starts_with("__") {
                    self.functions.insert(name.clone());
                }
                for a in args {
                    self.walk_expr(a);
                }
                for v in named_args.values() {
                    self.walk_expr(v);
                }
            }
            Expr::MethodCall {
                target,
                method,
                args,
                named_args,
            } => {
                self.type_methods.insert(method.clone());
                self.walk_expr(target);
                for a in args {
                    self.walk_expr(a);
                }
                for v in named_args.values() {
                    self.walk_expr(v);
                }
            }
            Expr::StringInterpolation { parts } => {
                for p in parts {
                    self.walk_expr(p);
                }
            }
            Expr::IndexAccess { target, index } => {
                self.walk_expr(target);
                self.walk_expr(index);
            }
            Expr::SliceAccess { target, start, end } => {
                self.walk_expr(target);
                if let Some(s) = start {
                    self.walk_expr(s);
                }
                if let Some(e) = end {
                    self.walk_expr(e);
                }
            }
            Expr::Match { value, arms } => {
                self.walk_expr(value);
                for arm in arms {
                    self.walk_expr(&arm.body);
                }
            }
            Expr::Unary { operand, .. } => {
                self.walk_expr(operand);
            }
            Expr::Binary { left, right, .. } => {
                self.walk_expr(left);
                self.walk_expr(right);
            }
            Expr::ProcedureLiteral { body, .. } => {
                for s in body {
                    self.walk_stmt(s);
                }
            }
            // Leaves — no children to walk
            Expr::Literal { .. }
            | Expr::VarGet { .. }
            | Expr::StaticGet { .. }
            | Expr::SharedArg { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Program;

    fn parse(source: &str) -> Program {
        let tokens = crate::lexer::Lexer::new(source).tokenize().unwrap();
        crate::parser::Parser::new(tokens).parse().unwrap()
    }

    #[test]
    fn test_simple_echo() {
        let program = parse("sys.echo(\"hello\")");
        let analysis = UsageAnalysis::from_program(&program);
        assert!(analysis.functions.contains("sys.echo"));
        assert!(analysis.namespaces.contains("sys"));
        assert!(analysis.required_features().is_empty());
    }

    #[test]
    fn test_http_get() {
        let program = parse("@result = http.get(\"https://example.com\")");
        let analysis = UsageAnalysis::from_program(&program);
        assert!(analysis.functions.contains("http.get"));
        assert!(analysis.namespaces.contains("http"));
        assert!(analysis
            .required_features()
            .contains(&"stdlib-http".to_string()));
    }

    #[test]
    fn test_type_method() {
        let program = parse("@x = \"hello\".len()");
        let analysis = UsageAnalysis::from_program(&program);
        assert!(analysis.type_methods.contains("len"));
    }

    #[test]
    fn test_no_features_for_core() {
        let program = parse("sys.echo(\"hi\")\n@x = math.add(1, 2)\n@f = fs.read(\"test.txt\")");
        let analysis = UsageAnalysis::from_program(&program);
        assert!(analysis.namespaces.contains("sys"));
        assert!(analysis.namespaces.contains("math"));
        assert!(analysis.namespaces.contains("fs"));
        assert!(analysis.required_features().is_empty());
    }

    #[test]
    fn test_multiple_features() {
        let program = parse("@h = http.get(\"url\")\n@c = crypto.hash(\"data\", \"sha256\")");
        let analysis = UsageAnalysis::from_program(&program);
        let features = analysis.required_features();
        assert!(features.contains(&"stdlib-http".to_string()));
        assert!(features.contains(&"stdlib-crypto".to_string()));
    }

    #[test]
    fn test_list_map_literals_not_functions() {
        let program = parse("@x = [1, 2, 3]\n@y = {\"a\": 1}");
        let analysis = UsageAnalysis::from_program(&program);
        // __list__ and __map__ should not appear in functions
        assert!(!analysis.functions.contains("__list__"));
        assert!(!analysis.functions.contains("__map__"));
    }
}
