use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_transpile_http_listen_compiles() {
    let source = r#"
        @hit_count = 0
        http_listen(port: 8080, @req, @resp, shared @hit_count) {
            @hit_count += 1
            @resp["body"] = "Hits: " + number.to_string(@hit_count)
            @headers = @resp["headers"]
            @headers["x-powered-by"] = "Corvo"
            @resp["headers"] = @headers
        }
    "#;

    let dir = tempdir().unwrap();
    let script_path = dir.path().join("http_test.corvo");
    fs::write(&script_path, source).unwrap();

    let output_dir = dir.path().join("http_test_project");

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

    assert!(status.success());
    assert!(output_dir.join("Cargo.toml").exists());
    assert!(output_dir.join("src/http_test.rs").exists());

    // Check that the transpiled code compiles successfully.
    // We don't use 'cargo run' because the server would block forever.
    let output = Command::new("cargo")
        .args(["check"])
        .current_dir(&output_dir)
        .output()
        .expect("failed to check transpiled project");

    if !output.status.success() {
        println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());
}
