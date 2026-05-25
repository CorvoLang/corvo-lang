#[cfg(all(unix, feature = "stdlib-pex"))]
mod unix_run_test {
    use corvo_lang::run_tests_file;
    use std::io::Write;
    use std::sync::Mutex;

    static CORVO_BIN_ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Serialize and restore process-global `CORVO_BIN` for parallel test safety.
    struct CorvoBinGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        previous: Option<std::ffi::OsString>,
    }

    impl CorvoBinGuard {
        fn set(path: &str) -> Self {
            let lock = CORVO_BIN_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let previous = std::env::var_os("CORVO_BIN");
            std::env::set_var("CORVO_BIN", path);
            Self {
                _lock: lock,
                previous,
            }
        }
    }

    impl Drop for CorvoBinGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var("CORVO_BIN", value),
                None => std::env::remove_var("CORVO_BIN"),
            }
        }
    }

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

        let _corvo_bin = CorvoBinGuard::set(env!("CARGO_BIN_EXE_corvo"));
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

        let _corvo_bin = CorvoBinGuard::set(env!("CARGO_BIN_EXE_corvo"));
        let err = run_tests_file(file.path()).expect_err("run_test should fail");
        assert!(format!("{err}").contains("failed"));
    }

    #[test]
    fn run_test_isolates_state_between_blocks() {
        let mut file = tempfile::Builder::new()
            .suffix(".corvo")
            .tempfile()
            .expect("temp file");
        write!(
            file,
            r#"
run_test("setup", [], @p) {{
    @tag = "polluted"
}}
run_test(@tag, [], @p) {{
    pex.exp_string(@p, "unused")
}}
"#
        )
        .expect("write temp script");
        file.flush().expect("flush");

        let _corvo_bin = CorvoBinGuard::set(env!("CARGO_BIN_EXE_corvo"));
        let err = run_tests_file(file.path()).expect_err("second test name should fail");
        let msg = format!("{err}");
        assert!(
            msg.contains("not found") || msg.contains("Variable"),
            "expected undefined @tag after per-test reset, got: {msg}"
        );
    }
}

#[test]
fn run_test_skipped_in_normal_interpreter() {
    let source = r#"
run_test("fail", [], @p) { assert_eq("1", "2") }
"#;
    corvo_lang::run_source(source).expect("run_test is a no-op in normal mode");
}
