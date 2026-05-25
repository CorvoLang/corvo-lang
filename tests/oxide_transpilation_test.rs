#[path = "common/mod.rs"]
mod common;

use corvo_lang::compiler::oxide_transpiler::OxideTranspiler;
use corvo_lang::compiler::usage_analyzer::UsageAnalysis;
use corvo_lang::lexer::tokenizer::Lexer;
use corvo_lang::parser::recursive_descent::Parser;
use std::fs;
use tempfile::tempdir;

fn with_oxide_cargo_lock<T>(f: impl FnOnce() -> T) -> T {
    let _guard = common::nested_cargo_lock().expect("nested cargo lock");
    f()
}

/// Path to the `corvo-lang` crate for generated Oxide projects.
///
/// Set `CORVO_LANG_LOCAL_PATH`, or run `cargo test` from the corvo-lang repo root (uses `current_dir`).
fn corvo_lang_path_for_tests() -> String {
    std::env::var("CORVO_LANG_LOCAL_PATH").unwrap_or_else(|_| {
        std::env::current_dir()
            .expect(
                "set CORVO_LANG_LOCAL_PATH to the corvo-lang repo root, or run tests from that directory",
            )
            .display()
            .to_string()
    })
}

fn corvo_lang_dep_toml(local_path: &str, features: &[String]) -> String {
    let path = local_path.replace('\\', "/");
    if features.is_empty() {
        format!(
            r#"corvo-lang = {{ path = "{}", default-features = false }}"#,
            path
        )
    } else {
        let list = features
            .iter()
            .map(|f| format!("\"{}\"", f))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"corvo-lang = {{ path = "{}", default-features = false, features = [{}] }}"#,
            path, list
        )
    }
}

fn prepare_oxide_temp_project(name: &str, source: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempdir().expect("Failed to create temp dir");
    let corvo_file = temp.path().join(format!("{}.corvo", name));
    fs::write(&corvo_file, source).expect("Failed to write corvo file");

    let output_dir = temp.path().join(format!("oxide_{}", name));

    let mut lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer.tokenize().expect("Tokenization failed"));
    let program = parser.parse().expect("Parsing failed");

    let usage = UsageAnalysis::from_program(&program);
    let features = usage.required_features();

    let transpiler = OxideTranspiler::new(usage);
    let rust_code = transpiler.transpile(&program);

    let src_dir = output_dir.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src dir");
    fs::write(src_dir.join("main.rs"), rust_code).expect("Failed to write main.rs");

    let mut deps = String::new();
    if features.contains(&"stdlib-http".to_string()) {
        deps.push_str("reqwest = { version = \"0.12\", features = [\"json\"] }\n");
    }
    if features.contains(&"stdlib-json".to_string())
        || features.contains(&"stdlib-http".to_string())
    {
        deps.push_str("serde = { version = \"1.0\", features = [\"derive\"] }\n");
        deps.push_str("serde_json = \"1.0\"\n");
    }

    let local_path = corvo_lang_path_for_tests();
    let corvo_dep = corvo_lang_dep_toml(&local_path, &features);
    let cargo_toml = format!(
        r#"
[package]
name = "oxide_{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}
tokio = {{ version = "1.0", features = ["full"] }}
{}
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
"#,
        name, corvo_dep, deps
    );
    fs::write(output_dir.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    (temp, output_dir)
}

/// Like `assert_oxide_exit`, but only runs `cargo check --release` (for programs that block on `cargo run`, e.g. `http_listen`).
fn assert_oxide_cargo_check_release(name: &str, source: &str) {
    with_oxide_cargo_lock(|| {
        let (_temp, output_dir) = prepare_oxide_temp_project(name, source);
        let status = common::nested_project_cargo()
            .expect("nested cargo target dir")
            .args(["check", "--release"])
            .current_dir(&output_dir)
            .status()
            .expect("cargo check (oxide project)");
        assert!(
            status.success(),
            "oxide-transpiled {} failed `cargo check --release`",
            name
        );
    });
}

fn assert_oxide_exit(name: &str, source: &str, expected_code: i32) {
    with_oxide_cargo_lock(|| {
        let (_temp, output_dir) = prepare_oxide_temp_project(name, source);

        let run_status = common::nested_project_cargo()
            .expect("nested cargo target dir")
            .arg("run")
            .arg("--release")
            .current_dir(&output_dir)
            .status()
            .expect("cargo run (oxide project)");

        assert_eq!(
            run_status.code().unwrap_or(-1),
            expected_code,
            "oxide-transpiled {} exit code",
            name
        );
    });
}

#[test]
fn test_oxide_basic_math_and_echo() {
    let source = r#"
        @a = 10
        @b = 20
        @c = math.add(@a, @b)
        sys.echo("Result: " + @c.to_string())
        if (@c == 30) {
            sys.exit(0)
        } else {
            sys.exit(1)
        }
    "#;
    assert_oxide_exit("oxide_basic", source, 0);
}

#[test]
fn test_oxide_string_methods() {
    let source = r#"
        @s = "hello world"
        if (@s.len() == 11 && @s.starts_with("hello") && @s.to_upper() == "HELLO WORLD") {
            sys.exit(0)
        } else {
            sys.exit(1)
        }
    "#;
    assert_oxide_exit("oxide_strings", source, 0);
}

#[test]
fn test_oxide_shared_vars() {
    let source = r#"
        @counter = 0
        loop {
            @counter = math.add(@counter, 1)
            if (@counter >= 5) {
                terminate
            }
        }
        if (@counter == 5) {
            sys.exit(0)
        } else {
            sys.exit(1)
        }
    "#;
    assert_oxide_exit("oxide_loop", source, 0);
}

#[test]
fn test_oxide_http_listen_mock() {
    // `http_listen` blocks accepting connections; like `http_listen_transpile_test`, only verify the Oxide output builds.
    let source = r#"
        http_listen(port: 0, @req, @resp) {
            sys.echo("Request: " + @req.to_string())
            terminate
        }
        sys.exit(0)
    "#;
    assert_oxide_cargo_check_release("oxide_http_trigger", source);
}

#[test]
fn test_transpile_procedure_exit_request_mismatch() {
    let source = r#"
        prep {}
        @p = procedure() {
          try {
            sys.exit(0)
          } fallback {}
        }
        @p.call()
    "#;
    assert_oxide_exit("transpile_repro", source, 0);
}

#[test]
fn test_transpile_try_error_propagation() {
    with_oxide_cargo_lock(|| {
        let source = r#"
        @p = procedure() {
          try {
            math.div(1, 0)
          } fallback {
            # Fail again
            math.div(2, 0)
          }
        }
        @p.call()
    "#;
        // We expect this to FAIL (exit code 1 or similar) because of division by zero.
        // If it's swallowed, it will return 0.

        let temp = tempdir().expect("Failed to create temp dir");
        let name = "transpile_propagate";
        let output_dir = temp.path().join(format!("oxide_{}", name));

        let mut tokenizer = Lexer::new(source);
        let mut parser = Parser::new(tokenizer.tokenize().expect("Tokenization failed"));
        let program = parser.parse().expect("Parsing failed");
        let usage = UsageAnalysis::from_program(&program);
        let features = usage.required_features();
        let transpiler = OxideTranspiler::new(usage);
        let rust_code = transpiler.transpile(&program);

        let src_dir = output_dir.join("src");
        fs::create_dir_all(&src_dir).expect("Failed to create src dir");
        fs::write(src_dir.join("main.rs"), rust_code).expect("Failed to write main.rs");

        let local_path = corvo_lang_path_for_tests();
        let corvo_dep = corvo_lang_dep_toml(&local_path, &features);
        let cargo_toml = format!(
            r#"
[package]
name = "oxide_{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}
tokio = {{ version = "1.0", features = ["full"] }}
{}
"#,
            name, corvo_dep, ""
        );
        fs::write(output_dir.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

        let status = common::nested_project_cargo()
            .expect("nested cargo target dir")
            .arg("run")
            .arg("--release")
            .current_dir(&output_dir)
            .status()
            .expect("Failed to run cargo build");

        assert!(
            !status.success(),
            "Error should have been propagated and caused non-zero exit code"
        );
    });
}

#[test]
fn test_transpile_fs_chown_propagation() {
    with_oxide_cargo_lock(|| {
        let source = r#"
        @p = procedure() {
          try {
            # This should fail if file doesn't exist
            fs.chown("/tmp/corvo-does-not-exist-123", -1, 0, true)
          } fallback {
            # Re-throw by calling something that fails?
            # Or just let it fail.
            fs.chown("/tmp/corvo-does-not-exist-456", -1, 0, true)
          }
        }
        @p.call()
    "#;

        let temp = tempdir().expect("Failed to create temp dir");
        let name = "transpile_fs_chown";
        let output_dir = temp.path().join(format!("oxide_{}", name));

        let mut lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer.tokenize().expect("Tokenization failed"));
        let program = parser.parse().expect("Parsing failed");
        let usage = UsageAnalysis::from_program(&program);
        let features = usage.required_features();
        let transpiler = OxideTranspiler::new(usage);
        let rust_code = transpiler.transpile(&program);

        let src_dir = output_dir.join("src");
        fs::create_dir_all(&src_dir).expect("Failed to create src dir");
        fs::write(src_dir.join("main.rs"), rust_code).expect("Failed to write main.rs");

        let local_path = corvo_lang_path_for_tests();
        let corvo_dep = corvo_lang_dep_toml(&local_path, &features);
        let cargo_toml = format!(
            r#"
[package]
name = "oxide_{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}
tokio = {{ version = "1.0", features = ["full"] }}
{}
"#,
            name, corvo_dep, ""
        );
        fs::write(output_dir.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

        let status = common::nested_project_cargo()
            .expect("nested cargo target dir")
            .arg("run")
            .arg("--release")
            .current_dir(&output_dir)
            .status()
            .expect("Failed to run cargo build");

        assert!(
            !status.success(),
            "fs.chown error should have been propagated"
        );
    });
}
