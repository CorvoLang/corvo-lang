use corvo_lang::compiler::Compiler;

use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_transpile_async_browse() {
    let dir = tempdir().expect("Failed to create temp dir");
    let script_path = dir.path().join("async_test.corvo");
    let project_path = dir.path().join("async_project");

    let source = r#"
        @nums = [1, 2, 3, 4, 5]
        @sum = 0
        
        @worker = procedure(@item, @s) {
            @s = math.add(@s, @item)
        }
        
        async_browse(@nums, @worker, @item, shared @sum)
        
        sys.echo(@sum)
    "#;

    fs::write(&script_path, source).expect("Failed to write script");

    let compiler = Compiler::new(source.to_string(), script_path.clone());
    // No need to call pre_execute if we don't have prep blocks,
    // but building without it might fail if expectations are there.
    // Actually, transpiler needs the pre_execute results (statics).

    compiler
        .transpile(&project_path)
        .expect("Transpilation failed");

    let output = Command::new("cargo")
        .arg("run")
        .current_dir(&project_path)
        .output()
        .expect("Failed to run transpiled project");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Project execution failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout.trim(), "15");
}
