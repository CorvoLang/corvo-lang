use std::fs;
use std::process::Command;
use tempfile::tempdir;

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

    let dir = tempdir().unwrap();
    let script_path = dir.path().join("fizzbuzz.corvo");
    fs::write(&script_path, source).unwrap();

    let output_dir = dir.path().join("fizzbuzz_project");

    // Run corvo --transpile
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
    assert!(output_dir.join("src/fizzbuzz.rs").exists());

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

    // Run the transpiled project
    let output = Command::new("cargo")
        .args(["run", "--bin", "fizzbuzz"])
        .current_dir(&output_dir)
        .output()
        .expect("failed to run transpiled project");

    if !output.status.success() {
        println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout
        .contains("1\n2\nFizz\n4\nBuzz\nFizz\n7\n8\nFizz\nBuzz\n11\nFizz\n13\n14\nFizzBuzz\n"));
}
