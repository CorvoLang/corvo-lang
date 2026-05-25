//! Regression tests for transpilation: issue #13 (`args.parse` + map literals), plus terminate and
//! related control-flow parity with the interpreter (GitHub Copilot / PR #24 follow-ups).

#[path = "common/mod.rs"]
mod common;

use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn transpiled_args_parse_short_value_space_same_as_glued() {
    let _nested_cargo = common::nested_cargo_lock().expect("nested cargo lock");
    let source = r#"
prep {}

@spec = {
  "aliases": { "w": "width" },
  "short_values": ["w"],
  "long_values": ["width"]
}
@parsed = args.parse(os.argv(), @spec)
sys.echo(
  number.to_string(list.len(@parsed["positional"]))
  + ":"
  + @parsed["options"].get("width", "")
)
"#;

    let dir = tempdir().unwrap();
    let script_path = dir.path().join("repro_args_parse.corvo");
    fs::write(&script_path, source).unwrap();
    let output_dir = dir.path().join("repro_args_parse_project");

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
    assert!(status.success(), "transpile failed");

    let cargo_toml_path = output_dir.join("Cargo.toml");
    let mut cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap();
    let repo_path = std::env::current_dir().unwrap();
    let repo_path = repo_path.to_string_lossy().replace('\\', "/");
    cargo_toml.push_str(&format!(
        "\n[patch.crates-io]\ncorvo-lang = {{ path = \"{repo_path}\" }}\n"
    ));
    fs::write(&cargo_toml_path, cargo_toml).unwrap();

    let out_space = common::nested_project_cargo()
        .expect("nested cargo target dir")
        .args(["run", "--quiet", "--", "-w", "80"])
        .current_dir(&output_dir)
        .output()
        .expect("cargo run -w 80");
    assert!(
        out_space.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out_space.stderr)
    );
    let line_space = String::from_utf8_lossy(&out_space.stdout)
        .trim()
        .to_string();
    assert_eq!(
        line_space, "0:80",
        "expected `0:80` (no stray positionals, width 80) for `-w 80`"
    );

    let out_glued = common::nested_project_cargo()
        .expect("nested cargo target dir")
        .args(["run", "--quiet", "--", "-w80"])
        .current_dir(&output_dir)
        .output()
        .expect("cargo run -w80");
    assert!(
        out_glued.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out_glued.stderr)
    );
    let line_glued: String = String::from_utf8_lossy(&out_glued.stdout)
        .trim()
        .to_string();
    assert_eq!(line_glued, "0:80", "expected `0:80` for `-w80`");
}

#[test]
fn transpiled_terminate_inside_procedure_exits_loop_cleanly() {
    let _nested_cargo = common::nested_cargo_lock().expect("nested cargo lock");
    let source = r#"
@g = procedure(@av, @out_av) {
  @out_av = []
  @i = 0
  @n = list.len(@av)
  loop {
    if (@i >= @n) { terminate }
    @out_av = list.push(@out_av, list.get(@av, @i))
    @i = @i + 1
  }
}
@acc = []
@g.call(os.argv(), @acc)
sys.echo(number.to_string(list.len(@acc)))
"#;

    let dir = tempdir().unwrap();
    let script_path = dir.path().join("repro_terminate_proc.corvo");
    fs::write(&script_path, source).unwrap();
    let output_dir = dir.path().join("repro_terminate_proc_project");

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
    assert!(status.success(), "transpile failed");

    let cargo_toml_path = output_dir.join("Cargo.toml");
    let mut cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap();
    let repo_path = std::env::current_dir().unwrap();
    let repo_path = repo_path.to_string_lossy().replace('\\', "/");
    cargo_toml.push_str(&format!(
        "\n[patch.crates-io]\ncorvo-lang = {{ path = \"{repo_path}\" }}\n"
    ));
    fs::write(&cargo_toml_path, cargo_toml).unwrap();

    let out = common::nested_project_cargo()
        .expect("nested cargo target dir")
        .args(["run", "--quiet", "--", "a", "b", "c"])
        .current_dir(&output_dir)
        .output()
        .expect("cargo run transpiled terminate repro");
    assert!(
        out.status.success(),
        "transpiled binary failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let line = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert_eq!(line, "3", "expected argv length copied by procedure");
}

/// Copilot PR #24: top-level `terminate` must skip subsequent statements in transpiled `run()`.
#[test]
fn transpiled_top_level_terminate_skips_following_statements() {
    let _nested_cargo = common::nested_cargo_lock().expect("nested cargo lock");
    let source = r#"
sys.echo("first")
terminate
sys.echo("second")
"#;

    let dir = tempdir().unwrap();
    let script_path = dir.path().join("repro_top_terminate.corvo");
    fs::write(&script_path, source).unwrap();
    let output_dir = dir.path().join("repro_top_terminate_project");

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
    assert!(status.success(), "transpile failed");

    let cargo_toml_path = output_dir.join("Cargo.toml");
    let mut cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap();
    let repo_path = std::env::current_dir().unwrap();
    let repo_path = repo_path.to_string_lossy().replace('\\', "/");
    cargo_toml.push_str(&format!(
        "\n[patch.crates-io]\ncorvo-lang = {{ path = \"{repo_path}\" }}\n"
    ));
    fs::write(&cargo_toml_path, cargo_toml).unwrap();

    let out = common::nested_project_cargo()
        .expect("nested cargo target dir")
        .args(["run", "--quiet", "--"])
        .current_dir(&output_dir)
        .output()
        .expect("cargo run transpiled top-level terminate");
    assert!(
        out.status.success(),
        "transpiled binary failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        combined.contains("first"),
        "expected first echo, got: {combined:?}"
    );
    assert!(
        !combined.contains("second"),
        "second echo must not run after terminate, got: {combined:?}"
    );
}
