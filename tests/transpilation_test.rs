#[path = "common/mod.rs"]
mod common;

use std::fs;
use std::process::Command;
use tempfile::tempdir;

/// Transpile `source` to a temp project named `stem`, patch in path dependency, run the binary.
fn transpile_and_run(stem: &str, source: &str) -> std::process::Output {
    let _nested_cargo = common::nested_cargo_lock().expect("nested cargo lock");
    let dir = tempdir().unwrap();
    let script_path = dir.path().join(format!("{stem}.corvo"));
    fs::write(&script_path, source).unwrap();
    let output_dir = dir.path().join("out");

    let status = Command::new("cargo")
        .args([
            "run",
            "--",
            "--transpile",
            script_path.to_str().unwrap(),
            "-o",
            output_dir.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run corvo --transpile");
    assert!(status.success(), "transpile failed for {stem}");

    let cargo_toml_path = output_dir.join("Cargo.toml");
    let mut cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap();
    let repo_path = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    cargo_toml.push_str(&format!(
        "\n[patch.crates-io]\ncorvo-lang = {{ path = \"{repo_path}\" }}\n",
    ));
    fs::write(&cargo_toml_path, cargo_toml).unwrap();

    common::nested_project_cargo()
        .expect("nested cargo target dir")
        .args(["run", "--bin", stem])
        .current_dir(&output_dir)
        .output()
        .expect("failed to run transpiled project")
}

fn assert_transpile_exit(stem: &str, source: &str, expected: i32) {
    assert_transpile_exit_with_forbidden(stem, source, expected, &[]);
}

fn assert_transpile_exit_with_forbidden(
    stem: &str,
    source: &str,
    expected: i32,
    must_not_contain: &[&str],
) {
    let out = transpile_and_run(stem, source);
    let code = out.status.code().unwrap_or(-1);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if code != expected {
        eprintln!("transpiled binary: {stem}");
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&out.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&out.stderr));
    }
    assert_eq!(code, expected, "transpiled {stem} exit code");
    for pat in must_not_contain {
        assert!(
            !combined.contains(pat),
            "expected output not to contain {pat:?}, got: {combined:?}"
        );
    }
}

#[test]
fn test_transpile_try_sys_exit_skips_fallback() {
    let source = r#"
        try {
            sys.exit(11)
        } fallback {
            sys.echo("SHOULD_NOT_RUN_FB")
            sys.exit(99)
        }
    "#;
    assert_transpile_exit_with_forbidden("try_exit_skip_fb", source, 11, &["SHOULD_NOT_RUN_FB"]);
}

#[test]
fn test_transpile_try_fallback_sys_exit() {
    let source = r#"
        try {
            assert_eq(1, 2)
        } fallback {
            sys.exit(13)
        }
    "#;
    assert_transpile_exit("try_exit_in_fb", source, 13);
}

#[test]
fn test_transpile_try_nested_sys_exit_skips_fallbacks() {
    let source = r#"
        try {
            try {
                sys.exit(17)
            } fallback {
                sys.exit(88)
            }
        } fallback {
            sys.exit(99)
        }
    "#;
    assert_transpile_exit("try_nested_exit", source, 17);
}

#[test]
fn test_transpile_try_first_fallback_exit_skips_second() {
    let source = r#"
        try {
            assert_eq(1, 2)
        } fallback {
            sys.exit(2)
        } fallback {
            sys.exit(77)
        }
    "#;
    assert_transpile_exit("try_fb_chain_exit", source, 2);
}

#[test]
fn test_transpile_try_first_fallback_ok_skips_second() {
    let source = r#"
        try {
            assert_eq(1, 2)
        } fallback {
            var.set("a", 1)
        } fallback {
            sys.exit(99)
        }
    "#;
    assert_transpile_exit("try_fb_ok_skip", source, 0);
}

#[test]
fn test_transpile_inner_fallback_sys_exit_skips_outer_fallback() {
    let source = r#"
        try {
            try {
                assert_eq(1, 2)
            } fallback {
                sys.exit(19)
            }
        } fallback {
            sys.echo("OUTER_FB_SHOULD_NOT_RUN")
            sys.exit(1)
        }
    "#;
    assert_transpile_exit_with_forbidden(
        "try_inner_fb_exit",
        source,
        19,
        &["OUTER_FB_SHOULD_NOT_RUN"],
    );
}

#[test]
fn test_transpile_and_run_fizzbuzz() {
    let source = r#"
        @i = 1
        loop {
            @done = false
            try {
                assert_gt(@i, 15)
                @done = true
            } fallback {}

            match(@done) {
                true => sys.exit(0),
                _ => 0
            }

            @fizzy = match(math.mod(@i, 15)) {
                0 => "FizzBuzz",
                _ => match(math.mod(@i, 3)) {
                    0 => "Fizz",
                    _ => match(math.mod(@i, 5)) {
                        0 => "Buzz",
                        _ => @i
                    }
                }
            }
            sys.echo(@fizzy)
            @i += 1
        }
    "#;

    let out = transpile_and_run("fizzbuzz", source);
    if !out.status.success() {
        println!("STDOUT: {}", String::from_utf8_lossy(&out.stdout));
        println!("STDERR: {}", String::from_utf8_lossy(&out.stderr));
    }
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout
        .contains("1\n2\nFizz\n4\nBuzz\nFizz\n7\n8\nFizz\nBuzz\n11\nFizz\n13\n14\nFizzBuzz\n"));
}

#[test]
fn test_transpile_exit_mismatch_repro() {
    let source = r#"
        prep {}
        @p = procedure() {
          try {
            sys.exit(0)
          } fallback {
            # This should be skipped on exit request
          }
        }
        @p.call()
    "#;
    assert_transpile_exit("repro_exit_mismatch", source, 0);
}
