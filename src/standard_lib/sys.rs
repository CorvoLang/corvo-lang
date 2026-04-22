use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;
use std::io::{self, Write};

pub fn echo(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.is_empty() {
        return Ok(Value::Null);
    }

    for arg in args {
        print!("{}", arg);
    }
    println!();
    io::stdout()
        .flush()
        .map_err(|e| CorvoError::io(e.to_string()))?;

    Ok(Value::Null)
}

pub fn printf(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.is_empty() {
        return Ok(Value::Null);
    }
    let format = args[0].as_string().ok_or_else(|| {
        CorvoError::invalid_argument("sys.printf requires a format string as the first argument")
    })?;

    let mut result = String::new();
    let mut arg_idx = 1;
    let mut chars = format.chars().peekable();

    #[allow(unused_assignments)]
    let mut list_args_backing: Vec<Value> = Vec::new();
    let format_args = if args.len() == 2 {
        if let Some(list) = args[1].as_list() {
            list_args_backing = list.clone();
            arg_idx = 0;
            &list_args_backing
        } else {
            &args[1..]
        }
    } else {
        &args[1..]
    };

    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.peek() {
                Some('%') => {
                    result.push('%');
                    chars.next();
                }
                Some('s') => {
                    let val = format_args.get(arg_idx).unwrap_or(&Value::Null);
                    result.push_str(&val.to_string());
                    arg_idx += 1;
                    chars.next();
                }
                Some('d') | Some('i') => {
                    let val = format_args.get(arg_idx).cloned().unwrap_or(Value::Null);
                    let n = match val {
                        Value::Number(n) => n,
                        Value::String(s) => s.parse().unwrap_or(0.0),
                        _ => 0.0,
                    };
                    result.push_str(&format!("{:.0}", n));
                    arg_idx += 1;
                    chars.next();
                }
                Some('f') => {
                    let val = format_args.get(arg_idx).cloned().unwrap_or(Value::Null);
                    let n = match val {
                        Value::Number(n) => n,
                        Value::String(s) => s.parse().unwrap_or(0.0),
                        _ => 0.0,
                    };
                    result.push_str(&format!("{}", n));
                    arg_idx += 1;
                    chars.next();
                }
                Some('x') => {
                    let val = format_args.get(arg_idx).cloned().unwrap_or(Value::Null);
                    let n = match val {
                        Value::Number(n) => n,
                        Value::String(s) => s.parse().unwrap_or(0.0),
                        _ => 0.0,
                    };
                    result.push_str(&format!("{:x}", n as u64));
                    arg_idx += 1;
                    chars.next();
                }
                _ => {
                    result.push('%');
                }
            }
        } else {
            result.push(c);
        }
    }

    print!("{}", result);
    io::stdout()
        .flush()
        .map_err(|e| CorvoError::io(e.to_string()))?;

    Ok(Value::Null)
}

/// Print to stdout without appending a trailing newline.
pub fn print_no_newline(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
) -> CorvoResult<Value> {
    for arg in args {
        print!("{}", arg);
    }
    io::stdout()
        .flush()
        .map_err(|e| CorvoError::io(e.to_string()))?;
    Ok(Value::Null)
}

/// Print to stderr with a trailing newline (mirrors sys.echo but writes to stderr).
pub fn eprint_newline(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    for arg in args {
        eprint!("{}", arg);
    }
    eprintln!();
    io::stderr()
        .flush()
        .map_err(|e| CorvoError::io(e.to_string()))?;
    Ok(Value::Null)
}

pub fn read_line(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        print!("{}", args[0]);
        io::stdout()
            .flush()
            .map_err(|e| CorvoError::io(e.to_string()))?;
    }

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| CorvoError::io(e.to_string()))?;

    input = input.trim_end_matches(['\n', '\r']).to_string();
    Ok(Value::String(input))
}

pub fn read_all(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "sys.read_all takes no arguments",
        ));
    }
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)
        .map_err(|e| CorvoError::io(e.to_string()))?;
    Ok(Value::String(input))
}

pub fn stdin_isatty(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "sys.stdin_isatty takes no arguments",
        ));
    }
    use std::io::IsTerminal;
    Ok(Value::Boolean(std::io::stdin().is_terminal()))
}

pub fn stdout_isatty(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "sys.stdout_isatty takes no arguments",
        ));
    }
    use std::io::IsTerminal;
    Ok(Value::Boolean(std::io::stdout().is_terminal()))
}

pub fn sleep(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if args.is_empty() {
        return Err(CorvoError::invalid_argument("sleep requires ms argument"));
    }

    let ms = args[0]
        .as_number()
        .ok_or_else(|| CorvoError::r#type("sleep requires a number"))?;
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));

    Ok(Value::Null)
}

/// Exit the process with the given status code (default `0`). Does not print a Corvo error.
pub fn exit_process(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let code = args
        .first()
        .and_then(|v| v.as_number())
        .map(|n| n as i32)
        .unwrap_or(0);
    Err(CorvoError::ExitRequest { code })
}

pub fn panic(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let msg = if args.is_empty() {
        "panic".to_string()
    } else {
        args[0].to_string()
    };

    Err(CorvoError::runtime(msg))
}

pub fn exec(args: &[Value], named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let cmd_list = args.first().and_then(|v| v.as_list()).ok_or_else(|| {
        CorvoError::invalid_argument(
            "sys.exec requires a list of strings as the command, e.g. [\"ls\", \"-la\"]",
        )
    })?;

    if cmd_list.is_empty() {
        return Err(CorvoError::invalid_argument(
            "sys.exec command list must not be empty",
        ));
    }

    let parts: Vec<&str> = cmd_list
        .iter()
        .map(|v| {
            v.as_string().map(|s| s.as_str()).ok_or_else(|| {
                CorvoError::invalid_argument("sys.exec command list elements must all be strings")
            })
        })
        .collect::<CorvoResult<Vec<&str>>>()?;

    let program = parts[0];
    let cmd_display = parts.join(" ");

    let mut command = std::process::Command::new(program);
    command.args(&parts[1..]);

    if let Some(cwd) = named_args.get("cwd").and_then(|v| v.as_string()) {
        command.current_dir(cwd);
    }

    if let Some(env_map) = named_args.get("env").and_then(|v| v.as_map()) {
        command.env_clear();
        for (key, val) in env_map {
            if let Some(val_str) = val.as_string() {
                command.env(key, val_str);
            }
        }
    }

    #[cfg(target_os = "linux")]
    if let Some(ctx) = named_args.get("context").and_then(|v| v.as_string()) {
        use std::os::unix::process::CommandExt;
        let ctx_bytes = ctx.as_bytes().to_vec();
        unsafe {
            command.pre_exec(move || {
                let path = b"/proc/self/attr/exec\0";
                let fd = libc::open(path.as_ptr() as *const libc::c_char, libc::O_WRONLY);
                if fd < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                let res = libc::write(
                    fd,
                    ctx_bytes.as_ptr() as *const libc::c_void,
                    ctx_bytes.len(),
                );
                libc::close(fd);
                if res < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    let has_input = named_args
        .get("input")
        .and_then(|v| v.as_string())
        .is_some();

    if has_input {
        command.stdin(std::process::Stdio::piped());
    }
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child = command.spawn().map_err(|e| CorvoError::io(e.to_string()))?;

    if let Some(input_str) = named_args.get("input").and_then(|v| v.as_string()) {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input_str.as_bytes())
                .map_err(|e| CorvoError::io(e.to_string()))?;
        }
    }

    let timeout_secs = named_args.get("timeout").and_then(|v| v.as_number());

    let output = if let Some(secs) = timeout_secs {
        let duration = std::time::Duration::from_secs(secs as u64);
        match wait_with_timeout(child, duration) {
            Ok(output) => output,
            Err(e) => return Err(CorvoError::runtime(format!("timeout: {}", e))),
        }
    } else {
        child
            .wait_with_output()
            .map_err(|e| CorvoError::io(e.to_string()))?
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    let check = named_args
        .get("check")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if check && code != 0 {
        return Err(CorvoError::runtime(format!(
            "command '{}' returned non-zero exit code {}",
            cmd_display, code
        )));
    }

    let mut result = HashMap::new();
    result.insert("stdout".to_string(), Value::String(stdout));
    result.insert("stderr".to_string(), Value::String(stderr));
    result.insert("code".to_string(), Value::Number(code as f64));

    Ok(Value::Map(result))
}

fn wait_with_timeout(
    child: std::process::Child,
    timeout: std::time::Duration,
) -> Result<std::process::Output, String> {
    let id = child.id();
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(e.to_string()),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err("process thread panicked".to_string())
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            let _ = std::process::Command::new("kill")
                .arg("-9")
                .arg(id.to_string())
                .output();
            Err("process timed out".to_string())
        }
    }
}

#[allow(unused_variables)]
pub fn chroot(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("sys.chroot requires a path"))?;

    #[cfg(unix)]
    {
        use std::ffi::CString;
        let c_path =
            CString::new(path.as_str()).map_err(|e| CorvoError::invalid_argument(e.to_string()))?;
        unsafe {
            if libc::chroot(c_path.as_ptr()) != 0 {
                return Err(CorvoError::io(format!(
                    "chroot failed: {}",
                    std::io::Error::last_os_error()
                )));
            }
            let root = CString::new("/").unwrap();
            if libc::chdir(root.as_ptr()) != 0 {
                return Err(CorvoError::io(format!(
                    "chdir / failed: {}",
                    std::io::Error::last_os_error()
                )));
            }
        }
        Ok(Value::Boolean(true))
    }
    #[cfg(not(unix))]
    {
        Err(CorvoError::runtime("sys.chroot is only supported on Unix"))
    }
}

#[allow(unused_variables)]
pub fn nice(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let inc = args
        .first()
        .and_then(|v| v.as_number())
        .ok_or_else(|| CorvoError::invalid_argument("sys.nice requires an increment number"))?;

    #[cfg(unix)]
    {
        unsafe {
            // nice() returns the new priority, but it can be -1 which is a valid value.
            // We must clear errno before and check it after.
            #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
            {
                *libc::__error() = 0;
            }
            #[cfg(target_os = "linux")]
            {
                *libc::__errno_location() = 0;
            }

            let _ = libc::nice(inc as libc::c_int);

            #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
            let err = *libc::__error();
            #[cfg(target_os = "linux")]
            let err = *libc::__errno_location();

            if err != 0 {
                return Err(CorvoError::io(format!(
                    "nice failed: {}",
                    std::io::Error::from_raw_os_error(err)
                )));
            }
        }
        Ok(Value::Boolean(true))
    }
    #[cfg(not(unix))]
    {
        Err(CorvoError::runtime("sys.nice is only supported on Unix"))
    }
}
pub fn sync(_args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    #[cfg(unix)]
    {
        unsafe {
            libc::sync();
        }
        Ok(Value::Boolean(true))
    }
    #[cfg(not(unix))]
    {
        Ok(Value::Boolean(true)) // Optional: implement flush on windows?
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn empty_args() -> HashMap<String, Value> {
        HashMap::new()
    }

    #[test]
    fn test_echo() {
        let args = vec![Value::String("hello".to_string())];
        assert_eq!(echo(&args, &empty_args()).unwrap(), Value::Null);
    }

    #[test]
    fn test_echo_no_args() {
        assert_eq!(echo(&[], &empty_args()).unwrap(), Value::Null);
    }

    #[test]
    fn test_echo_multiple_args() {
        let args = vec![
            Value::String("hello".to_string()),
            Value::String(" world".to_string()),
        ];
        assert_eq!(echo(&args, &empty_args()).unwrap(), Value::Null);
    }

    #[test]
    fn test_echo_number() {
        let args = vec![Value::Number(42.0)];
        assert_eq!(echo(&args, &empty_args()).unwrap(), Value::Null);
    }

    #[test]
    fn test_panic_with_message() {
        let args = vec![Value::String("error msg".to_string())];
        let err = panic(&args, &empty_args()).unwrap_err();
        assert!(format!("{}", err).contains("error msg"));
    }

    #[test]
    fn test_panic_no_args() {
        let err = panic(&[], &empty_args()).unwrap_err();
        assert!(format!("{}", err).contains("panic"));
    }

    #[test]
    fn test_sleep_zero() {
        let args = vec![Value::Number(0.0)];
        assert_eq!(sleep(&args, &empty_args()).unwrap(), Value::Null);
    }

    #[test]
    fn test_sleep_no_args() {
        assert!(sleep(&[], &empty_args()).is_err());
    }

    #[test]
    fn test_sleep_wrong_type() {
        let args = vec![Value::String("100".to_string())];
        assert!(sleep(&args, &empty_args()).is_err());
    }

    // sys.exec tests

    fn make_cmd(parts: &[&str]) -> Value {
        Value::List(parts.iter().map(|s| Value::String(s.to_string())).collect())
    }

    #[test]
    fn test_exec_basic() {
        let args = vec![make_cmd(&["echo", "hello"])];
        let result = exec(&args, &empty_args()).unwrap();
        match result {
            Value::Map(m) => {
                let stdout = m.get("stdout").unwrap().as_string().unwrap();
                assert!(stdout.contains("hello"));
                let code = m.get("code").unwrap().as_number().unwrap();
                assert_eq!(code, 0.0);
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_exec_with_input() {
        let args = vec![make_cmd(&["cat"])];
        let mut named = HashMap::new();
        named.insert("input".to_string(), Value::String("pipe input".to_string()));
        let result = exec(&args, &named).unwrap();
        match result {
            Value::Map(m) => {
                let stdout = m.get("stdout").unwrap().as_string().unwrap();
                assert_eq!(stdout, "pipe input");
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_exec_with_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = std::fs::canonicalize(dir.path()).unwrap();
        let cwd_s = cwd.to_string_lossy().to_string();
        let marker = cwd
            .file_name()
            .expect("temp dir must have a name")
            .to_string_lossy()
            .to_string();

        #[cfg(unix)]
        let args = vec![make_cmd(&["pwd"])];
        #[cfg(windows)]
        let args = vec![make_cmd(&["cmd", "/C", "echo", "%CD%"])];

        let mut named = HashMap::new();
        named.insert("cwd".to_string(), Value::String(cwd_s.clone()));
        let result = exec(&args, &named).unwrap();
        match result {
            Value::Map(m) => {
                let code = m.get("code").unwrap().as_number().unwrap();
                assert_eq!(code, 0.0);
                let stdout = m.get("stdout").unwrap().as_string().unwrap();
                let out = stdout.trim().trim_end_matches('\r');
                assert!(
                    out.contains(&marker) || out.eq_ignore_ascii_case(&cwd_s),
                    "stdout={stdout:?} cwd={cwd_s:?} marker={marker:?}"
                );
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_exec_with_env() {
        let args = vec![make_cmd(&["printenv", "CORVO_TEST_EXEC_VAR"])];
        let mut env_map = HashMap::new();
        env_map.insert(
            "CORVO_TEST_EXEC_VAR".to_string(),
            Value::String("env_value".to_string()),
        );
        let mut named = HashMap::new();
        named.insert("env".to_string(), Value::Map(env_map));
        let result = exec(&args, &named).unwrap();
        match result {
            Value::Map(m) => {
                let stdout = m.get("stdout").unwrap().as_string().unwrap();
                assert!(stdout.contains("env_value"));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_exec_check_passes() {
        let args = vec![make_cmd(&["true"])];
        let mut named = HashMap::new();
        named.insert("check".to_string(), Value::Boolean(true));
        assert!(exec(&args, &named).is_ok());
    }

    #[test]
    fn test_exec_check_fails() {
        let args = vec![make_cmd(&["false"])];
        let mut named = HashMap::new();
        named.insert("check".to_string(), Value::Boolean(true));
        let err = exec(&args, &named).unwrap_err();
        assert!(format!("{}", err).contains("non-zero exit code"));
    }

    #[test]
    fn test_exec_no_command() {
        assert!(exec(&[], &empty_args()).is_err());
    }

    #[test]
    fn test_exec_empty_list() {
        let args = vec![Value::List(vec![])];
        assert!(exec(&args, &empty_args()).is_err());
    }

    #[test]
    fn test_exec_non_string_element() {
        let args = vec![Value::List(vec![
            Value::String("echo".to_string()),
            Value::Number(42.0),
        ])];
        assert!(exec(&args, &empty_args()).is_err());
    }

    #[test]
    fn test_exec_timeout() {
        let args = vec![make_cmd(&["sleep", "10"])];
        let mut named = HashMap::new();
        named.insert("timeout".to_string(), Value::Number(1.0));
        let result = exec(&args, &named);
        assert!(result.is_err());
    }

    #[test]
    fn test_exec_nonzero_exit() {
        let args = vec![make_cmd(&["sh", "-c", "exit 42"])];
        let result = exec(&args, &empty_args()).unwrap();
        match result {
            Value::Map(m) => {
                let code = m.get("code").unwrap().as_number().unwrap();
                assert_eq!(code, 42.0);
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_exec_multiple_args() {
        let args = vec![make_cmd(&["echo", "hello", "world"])];
        let result = exec(&args, &empty_args()).unwrap();
        match result {
            Value::Map(m) => {
                let stdout = m.get("stdout").unwrap().as_string().unwrap();
                assert!(stdout.contains("hello"));
                assert!(stdout.contains("world"));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_exec_not_a_list() {
        let args = vec![Value::String("echo hello".to_string())];
        assert!(exec(&args, &empty_args()).is_err());
    }
}
