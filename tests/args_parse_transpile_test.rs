//! Regression test for GitHub issue #13: transpiled binaries must preserve map literals used
//! as `args.parse` specs so short options with a separate value token (e.g. `-w 80`) bind correctly.

use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn transpiled_args_parse_short_value_space_same_as_glued() {
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

    let out_space = Command::new("cargo")
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

    let out_glued = Command::new("cargo")
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
