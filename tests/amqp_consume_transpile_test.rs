use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_transpile_amqp_consume_compiles() {
    let source = r#"
        @messages_received = 0
        @conn_url = "amqp://127.0.0.1:5672/%2f"
        
        try {
            @conn = amqp.connect(@conn_url)
        } fallback {
            sys.exit(1)
        }
        
        amqp_consume(@conn, "test_queue", @msg, shared @messages_received) {
            @messages_received += 1
            sys.print("Received: ", @msg["body"])
            if (@messages_received >= 5) {
                sys.exit(0)
            }
        }
    "#;

    let dir = tempdir().unwrap();
    let script_path = dir.path().join("amqp_test.corvo");
    fs::write(&script_path, source).unwrap();

    let output_dir = dir.path().join("amqp_test_project");

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
    assert!(output_dir.join("src/amqp_test.rs").exists());

    // Patch Cargo.toml to use the local corvo-lang dependency
    let cargo_toml_path = output_dir.join("Cargo.toml");
    let mut cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap();
    let repo_dir = std::env::current_dir().unwrap();
    let repo_path = repo_dir.to_string_lossy().replace('\\', "/");
    cargo_toml.push_str(&format!(
        "\n[patch.crates-io]\ncorvo-lang = {{ path = \"{}\" }}\n",
        repo_path
    ));
    fs::write(&cargo_toml_path, cargo_toml).unwrap();

    // Check that the transpiled code compiles successfully.
    // We don't use 'cargo run' because the AMQP consumer loop would block forever
    // and would fail without a broker.
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
