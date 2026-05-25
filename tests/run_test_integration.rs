#[cfg(all(unix, feature = "stdlib-pex"))]
mod unix_run_test {
    use corvo_lang::run_tests_file;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn run_test_spawns_self_and_asserts_output() {
        let mut file = NamedTempFile::new().expect("temp file");
        write!(
            file,
            r#"
browse(os.argv(), @i, @arg) {{
    if @arg == "--ping" {{
        sys.echo("pong")
    }}
}}

run_test("ping", ["--ping"], @pex) {{
    @out = pex.exp_string(@pex, "pong")
    assert_eq("pong", map.get(@out, "before"))
}}
"#
        )
        .expect("write temp script");
        file.flush().expect("flush");

        run_tests_file(file.path()).expect("run_test should pass");
    }

    #[test]
    fn run_test_reports_failure() {
        let mut file = NamedTempFile::new().expect("temp file");
        write!(
            file,
            r#"
run_test("bad", [], @pex) {{
    assert_eq("1", "2")
}}
"#
        )
        .expect("write temp script");
        file.flush().expect("flush");

        let err = run_tests_file(file.path()).expect_err("run_test should fail");
        assert!(format!("{err}").contains("failed"));
    }
}

#[test]
fn run_test_skipped_in_normal_interpreter() {
    let source = r#"
run_test("fail", [], @p) { assert_eq("1", "2") }
"#;
    corvo_lang::run_source(source).expect("run_test is a no-op in normal mode");
}
