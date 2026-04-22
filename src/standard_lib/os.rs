use crate::runtime::RuntimeState;
use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

pub fn get_env(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let key = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("get_env requires a string key"))?;

    let default = args
        .get(1)
        .and_then(|v| v.as_string())
        .cloned()
        .unwrap_or_default();

    Ok(Value::String(std::env::var(key).unwrap_or(default)))
}

pub fn set_env(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let key = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("set_env requires a string key"))?;

    let value = args
        .get(1)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("set_env requires a string value"))?;

    std::env::set_var(key, value);
    Ok(Value::Null)
}

pub fn exec(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let cmd = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("exec requires a command string"))?;

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| CorvoError::io(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "stdout".to_string(),
        Value::String(String::from_utf8_lossy(&output.stdout).to_string()),
    );
    result.insert(
        "stderr".to_string(),
        Value::String(String::from_utf8_lossy(&output.stderr).to_string()),
    );
    result.insert(
        "code".to_string(),
        Value::Number(output.status.code().unwrap_or(-1) as f64),
    );

    Ok(Value::Map(result))
}

pub fn argv(
    args: &[Value],
    _named_args: &HashMap<String, Value>,
    state: &RuntimeState,
) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "os.argv does not accept arguments",
        ));
    }
    Ok(Value::List(
        state
            .script_argv()
            .iter()
            .map(|s| Value::String(s.clone()))
            .collect(),
    ))
}

/// Current working directory (POSIX `getcwd`).
pub fn getcwd(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument("os.getcwd takes no arguments"));
    }
    std::env::current_dir()
        .map(|p| Value::String(p.to_string_lossy().to_string()))
        .map_err(|e| CorvoError::io(e.to_string()))
}

pub fn temp_dir(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "os.temp_dir takes no arguments",
        ));
    }
    Ok(Value::String(
        std::env::temp_dir().to_string_lossy().to_string(),
    ))
}

pub fn info(_args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let mut result = HashMap::new();
    result.insert(
        "os".to_string(),
        Value::String(std::env::consts::OS.to_string()),
    );
    result.insert(
        "arch".to_string(),
        Value::String(std::env::consts::ARCH.to_string()),
    );
    result.insert(
        "hostname".to_string(),
        Value::String(
            hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
        ),
    );

    Ok(Value::Map(result))
}

/// Get all environment variables.
pub fn environ(_args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let mut env = HashMap::new();
    for (key, value) in std::env::vars() {
        env.insert(key, Value::String(value));
    }
    Ok(Value::Map(env))
}

/// Get the supplementary groups of the current user.
pub fn groups(_args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    #[cfg(unix)]
    {
        let mut groups = Vec::new();
        let uid = uzers::get_current_uid();
        if let Some(user) = uzers::get_user_by_uid(uid) {
            if let Some(glist) = uzers::get_user_groups(user.name(), user.primary_group_id()) {
                for g in glist {
                    groups.push(Value::String(g.name().to_string_lossy().to_string()));
                }
            }
        }
        Ok(Value::List(groups))
    }
    #[cfg(not(unix))]
    {
        Ok(Value::List(vec![]))
    }
}

/// Get the numeric host identifier.
pub fn hostid(_args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    #[cfg(unix)]
    {
        unsafe { Ok(Value::String(format!("{:08x}", libc::gethostid()))) }
    }
    #[cfg(not(unix))]
    {
        Ok(Value::String("00000000".to_string()))
    }
}

/// Get the number of processing units available.
pub fn nproc(_args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    Ok(Value::Number(count as f64))
}

/// Get disk space usage for a path.
#[allow(unused_variables)]
pub fn df(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let path = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.as_str())
        .unwrap_or(".");

    #[cfg(unix)]
    {
        use std::ffi::CString;
        let c_path = CString::new(path).map_err(|e| CorvoError::invalid_argument(e.to_string()))?;
        let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
        unsafe {
            if libc::statvfs(c_path.as_ptr(), &mut stats) != 0 {
                return Err(CorvoError::io(format!(
                    "statvfs failed: {}",
                    std::io::Error::last_os_error()
                )));
            }
        }
        let mut m = HashMap::new();
        m.insert(
            "total".to_string(),
            Value::Number((stats.f_blocks as f64) * (stats.f_frsize as f64)),
        );
        m.insert(
            "free".to_string(),
            Value::Number((stats.f_bfree as f64) * (stats.f_frsize as f64)),
        );
        m.insert(
            "available".to_string(),
            Value::Number((stats.f_bavail as f64) * (stats.f_frsize as f64)),
        );
        m.insert("bsize".to_string(), Value::Number(stats.f_frsize as f64));
        m.insert("blocks".to_string(), Value::Number(stats.f_blocks as f64));
        m.insert("bfree".to_string(), Value::Number(stats.f_bfree as f64));
        m.insert("bavail".to_string(), Value::Number(stats.f_bavail as f64));
        Ok(Value::Map(m))
    }
    #[cfg(not(unix))]
    {
        Err(CorvoError::runtime("os.df is only supported on Unix"))
    }
}
/// Returns a number representing how long the system has been running.
pub fn uptime(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument("os.uptime takes no arguments"));
    }

    #[cfg(target_os = "linux")]
    {
        let uptime_secs = std::fs::read_to_string("/proc/uptime")
            .map_err(|e| CorvoError::io(format!("failed to read /proc/uptime: {}", e)))?
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| CorvoError::runtime("failed to parse /proc/uptime"))?;

        Ok(Value::Number(uptime_secs))
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, use sysctl to get boot time and compute uptime
        let output = std::process::Command::new("sysctl")
            .args(["-n", "kern.boottime"])
            .output()
            .map_err(|e| CorvoError::io(format!("failed to run sysctl: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let boot_sec = stdout
            .split("sec = ")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.trim().parse::<f64>().ok())
            .ok_or_else(|| CorvoError::runtime("failed to parse sysctl boot time"))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| CorvoError::runtime(e.to_string()))?
            .as_secs_f64();

        Ok(Value::Number(now - boot_sec))
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use PowerShell to get uptime
        let output = std::process::Command::new("powershell")
            .args([
                "-Command",
                "(Get-Date) - (Get-CimInstance Win32_OperatingSystem).LastBootUpTime | Select-Object -ExpandProperty TotalSeconds",
            ])
            .output()
            .map_err(|e| CorvoError::io(format!("failed to get uptime: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let secs: f64 = stdout
            .trim()
            .parse()
            .map_err(|_| CorvoError::runtime("failed to parse uptime"))?;

        return Ok(Value::Number(secs));
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    Err(CorvoError::runtime(
        "os.uptime is not supported on this platform",
    ))
}

/// Get system load averages (1, 5, 15 minute).
/// Returns a map with keys "1min", "5min", "15min".
pub fn load_average(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "os.load_average takes no arguments",
        ));
    }

    #[cfg(target_os = "linux")]
    {
        let loadavg = std::fs::read_to_string("/proc/loadavg")
            .map_err(|e| CorvoError::io(format!("failed to read /proc/loadavg: {}", e)))?;

        let parts: Vec<&str> = loadavg.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(CorvoError::runtime("failed to parse /proc/loadavg"));
        }

        let mut result = HashMap::new();
        result.insert(
            "1min".to_string(),
            Value::Number(parts[0].parse().unwrap_or(0.0)),
        );
        result.insert(
            "5min".to_string(),
            Value::Number(parts[1].parse().unwrap_or(0.0)),
        );
        result.insert(
            "15min".to_string(),
            Value::Number(parts[2].parse().unwrap_or(0.0)),
        );

        Ok(Value::Map(result))
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("sysctl")
            .args(["-n", "vm.loadavg"])
            .output()
            .map_err(|e| CorvoError::io(format!("failed to run sysctl: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Output format: "{ 1.23 4.56 7.89 }"
        let nums: Vec<f64> = stdout
            .replace(['{', '}'], "")
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut result = HashMap::new();
        result.insert(
            "1min".to_string(),
            Value::Number(*nums.first().unwrap_or(&0.0)),
        );
        result.insert(
            "5min".to_string(),
            Value::Number(*nums.get(1).unwrap_or(&0.0)),
        );
        result.insert(
            "15min".to_string(),
            Value::Number(*nums.get(2).unwrap_or(&0.0)),
        );

        Ok(Value::Map(result))
    }

    #[cfg(target_os = "windows")]
    {
        // Windows doesn't have the same concept of load average
        // Return a map with 0 values and a note
        let mut result = HashMap::new();
        result.insert("1min".to_string(), Value::Number(0.0));
        result.insert("5min".to_string(), Value::Number(0.0));
        result.insert("15min".to_string(), Value::Number(0.0));
        return Ok(Value::Map(result));
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    Err(CorvoError::runtime(
        "os.load_average is not supported on this platform",
    ))
}

/// Get information about logged-in users.
/// Returns a list of maps: {"user": string, "line": string, "host": string, "time": number}
pub fn users(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument("os.users takes no arguments"));
    }

    #[cfg(unix)]
    {
        let mut users = Vec::new();
        unsafe {
            libc::setutxent();
            loop {
                let ut_ptr = libc::getutxent();
                if ut_ptr.is_null() {
                    break;
                }
                let ut = &*ut_ptr;
                if ut.ut_type == libc::USER_PROCESS {
                    let mut m = HashMap::new();

                    // Helper to convert fixed-size char arrays to String
                    let to_string = |bytes: &[libc::c_char]| {
                        let len = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
                        String::from_utf8_lossy(std::slice::from_raw_parts(
                            bytes.as_ptr() as *const u8,
                            len,
                        ))
                        .to_string()
                    };

                    m.insert("user".to_string(), Value::String(to_string(&ut.ut_user)));
                    m.insert("line".to_string(), Value::String(to_string(&ut.ut_line)));
                    m.insert("host".to_string(), Value::String(to_string(&ut.ut_host)));
                    m.insert("time".to_string(), Value::Number(ut.ut_tv.tv_sec as f64));
                    users.push(Value::Map(m));
                }
            }
            libc::endutxent();
        }
        Ok(Value::List(users))
    }

    #[cfg(not(unix))]
    {
        // Fallback for non-Unix: return empty list or mock
        Ok(Value::List(vec![]))
    }
}

/// Get the number of logged-in users.
/// Returns a number.
pub fn user_count(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "os.user_count takes no arguments",
        ));
    }

    #[cfg(unix)]
    {
        match users(&[], &HashMap::new())? {
            Value::List(l) => Ok(Value::Number(l.len() as f64)),
            _ => Ok(Value::Number(0.0)),
        }
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use PowerShell to get user count (if unavoidable)
        // or just return 1 for the current user.
        // Given the "no exec" rule, we'll try to use a native winapi if possible
        // but for now 1 is a safe bet for a desktop system.
        Ok(Value::Number(1.0))
    }

    #[cfg(not(any(unix, target_os = "windows")))]
    Err(CorvoError::runtime(
        "os.user_count is not supported on this platform",
    ))
}

/// Resolve a POSIX login name to a UID (for `chown`-style tools).
#[cfg(unix)]
pub fn user_id(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let name = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("os.user_id requires a user name"))?;
    let u = uzers::get_user_by_name(name).ok_or_else(|| {
        CorvoError::invalid_argument(format!("os.user_id: unknown user '{name}'"))
    })?;
    Ok(Value::Number(u.uid() as f64))
}

#[cfg(not(unix))]
pub fn user_id(_args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    Err(CorvoError::runtime("os.user_id is only supported on Unix"))
}

/// Resolve a POSIX group name to a GID (for `chgrp` / `chown`).
#[cfg(unix)]
pub fn group_id(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let name = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("os.group_id requires a group name"))?;
    let g = uzers::get_group_by_name(name).ok_or_else(|| {
        CorvoError::invalid_argument(format!("os.group_id: unknown group '{name}'"))
    })?;
    Ok(Value::Number(g.gid() as f64))
}

#[cfg(not(unix))]
pub fn group_id(_args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    Err(CorvoError::runtime("os.group_id is only supported on Unix"))
}

/// Get the current user's login name.
pub fn username(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "os.username takes no arguments",
        ));
    }

    #[cfg(unix)]
    {
        Ok(Value::String(
            uzers::get_current_username()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        ))
    }

    #[cfg(not(unix))]
    {
        Ok(Value::String(
            std::env::var("USERNAME")
                .or_else(|_| std::env::var("USER"))
                .unwrap_or_else(|_| "unknown".to_string()),
        ))
    }
}

/// Get terminal settings (mode).
/// Returns a map of settings.
#[cfg(unix)]
pub fn tty_get_mode(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "os.tty_get_mode takes no arguments",
        ));
    }
    let fd = 0; // stdin
    unsafe {
        let mut termios = std::mem::MaybeUninit::<libc::termios>::uninit();
        if libc::tcgetattr(fd, termios.as_mut_ptr()) != 0 {
            return Err(CorvoError::runtime("failed to get tty attributes"));
        }
        let termios = termios.assume_init();
        let mut result = HashMap::new();
        result.insert("iflag".to_string(), Value::Number(termios.c_iflag as f64));
        result.insert("oflag".to_string(), Value::Number(termios.c_oflag as f64));
        result.insert("cflag".to_string(), Value::Number(termios.c_cflag as f64));
        result.insert("lflag".to_string(), Value::Number(termios.c_lflag as f64));
        Ok(Value::Map(result))
    }
}

/// Set terminal settings (mode).
#[cfg(unix)]
pub fn tty_set_mode(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let mode = args
        .first()
        .and_then(|v| v.as_map())
        .ok_or_else(|| CorvoError::invalid_argument("os.tty_set_mode requires a map"))?;
    let fd = 0; // stdin
    unsafe {
        let mut termios = std::mem::MaybeUninit::<libc::termios>::uninit();
        if libc::tcgetattr(fd, termios.as_mut_ptr()) != 0 {
            return Err(CorvoError::runtime("failed to get tty attributes"));
        }
        let mut termios = termios.assume_init();

        if let Some(v) = mode.get("iflag").and_then(|v| v.as_number()) {
            termios.c_iflag = v as libc::tcflag_t;
        }
        if let Some(v) = mode.get("oflag").and_then(|v| v.as_number()) {
            termios.c_oflag = v as libc::tcflag_t;
        }
        if let Some(v) = mode.get("cflag").and_then(|v| v.as_number()) {
            termios.c_cflag = v as libc::tcflag_t;
        }
        if let Some(v) = mode.get("lflag").and_then(|v| v.as_number()) {
            termios.c_lflag = v as libc::tcflag_t;
        }

        if libc::tcsetattr(fd, libc::TCSANOW, &termios) != 0 {
            return Err(CorvoError::runtime("failed to set tty attributes"));
        }
        Ok(Value::Boolean(true))
    }
}

#[cfg(not(unix))]
pub fn tty_get_mode(_args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    Err(CorvoError::runtime(
        "os.tty_get_mode is only supported on Unix",
    ))
}

#[cfg(not(unix))]
pub fn tty_set_mode(_args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    Err(CorvoError::runtime(
        "os.tty_set_mode is only supported on Unix",
    ))
}

/// Get the name of the terminal connected to standard input.
#[cfg(unix)]
pub fn ttyname(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "os.ttyname takes no arguments",
        ));
    }
    let fd = 0; // stdin
    unsafe {
        let ptr = libc::ttyname(fd);
        if ptr.is_null() {
            return Err(CorvoError::runtime("not a tty"));
        }
        let c_str = std::ffi::CStr::from_ptr(ptr);
        Ok(Value::String(c_str.to_string_lossy().to_string()))
    }
}

#[cfg(not(unix))]
pub fn ttyname(_args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    Err(CorvoError::runtime("os.ttyname is only supported on Unix"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::RuntimeState;

    fn empty_args() -> HashMap<String, Value> {
        HashMap::new()
    }

    #[test]
    fn test_get_env_existing() {
        std::env::set_var("CORVO_TEST_VAR", "test_value");
        let args = vec![Value::String("CORVO_TEST_VAR".to_string())];
        assert_eq!(
            get_env(&args, &empty_args()).unwrap(),
            Value::String("test_value".to_string())
        );
        std::env::remove_var("CORVO_TEST_VAR");
    }

    #[test]
    fn test_get_env_default() {
        let args = vec![
            Value::String("CORVO_NONEXISTENT_VAR_XYZ".to_string()),
            Value::String("default_val".to_string()),
        ];
        assert_eq!(
            get_env(&args, &empty_args()).unwrap(),
            Value::String("default_val".to_string())
        );
    }

    #[test]
    fn test_get_env_empty_default() {
        let args = vec![Value::String("CORVO_NONEXISTENT_VAR_XYZ".to_string())];
        assert_eq!(
            get_env(&args, &empty_args()).unwrap(),
            Value::String("".to_string())
        );
    }

    #[test]
    fn test_set_env() {
        let args = vec![
            Value::String("CORVO_TEST_SET_XYZ".to_string()),
            Value::String("set_value".to_string()),
        ];
        assert_eq!(set_env(&args, &empty_args()).unwrap(), Value::Null);
        assert_eq!(std::env::var("CORVO_TEST_SET_XYZ").unwrap(), "set_value");
        std::env::remove_var("CORVO_TEST_SET_XYZ");
    }

    #[test]
    fn test_exec_echo() {
        let args = vec![Value::String("echo hello".to_string())];
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
    fn test_exec_returns_status_code() {
        let args = vec![Value::String("exit 42".to_string())];
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
    fn test_info() {
        let result = info(&[], &empty_args()).unwrap();
        match result {
            Value::Map(m) => {
                assert!(m.contains_key("os"));
                assert!(m.contains_key("arch"));
                assert!(m.contains_key("hostname"));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_info_os_value() {
        let result = info(&[], &empty_args()).unwrap();
        match result {
            Value::Map(m) => {
                let os_val = m.get("os").unwrap().as_string().unwrap();
                assert!(!os_val.is_empty());
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_argv_empty() {
        let state = RuntimeState::new();
        let result = argv(&[], &empty_args(), &state).unwrap();
        assert_eq!(result, Value::List(vec![]));
    }

    #[test]
    fn test_argv_script_args() {
        let mut state = RuntimeState::new();
        state.set_script_argv(vec!["a".to_string(), "b".to_string()]);
        let result = argv(&[], &empty_args(), &state).unwrap();
        assert_eq!(
            result,
            Value::List(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string())
            ])
        );
    }

    #[test]
    fn test_argv_rejects_args() {
        let state = RuntimeState::new();
        let args = vec![Value::String("x".to_string())];
        assert!(argv(&args, &empty_args(), &state).is_err());
    }
}
