#[cfg(all(unix, feature = "stdlib-pex"))]
mod unix_run_test {
    use corvo_lang::run_tests_file;
    use std::io::Write;

    #[test]
    fn run_test_spawns_self_and_asserts_output() {
        let mut file = tempfile::Builder::new()
            .suffix(".corvo")
            .tempfile()
            .expect("temp file");
        write!(
            file,
            r#"
@args = os.argv()
if (list.len(@args) > 0 && list.get(@args, 0) == "--ping") {{
    sys.echo("pong")
}}

run_test("ping", ["--ping"], @pex) {{
    pex.exp_string(@pex, "pong")
}}
"#
        )
        .expect("write temp script");
        file.flush().expect("flush");

        std::env::set_var("CORVO_BIN", env!("CARGO_BIN_EXE_corvo"));
        run_tests_file(file.path()).expect("run_test should pass");
    }

    #[test]
    fn run_test_reports_failure() {
        let mut file = tempfile::Builder::new()
            .suffix(".corvo")
            .tempfile()
            .expect("temp file");
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

        std::env::set_var("CORVO_BIN", env!("CARGO_BIN_EXE_corvo"));
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
