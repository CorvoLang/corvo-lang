use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Format a Unix timestamp in the **local** timezone (honours `TZ`) using a `strftime` pattern.
/// Args: `seconds: number`, `[nanoseconds: number]`, `format: string`
pub fn format_local(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let secs = args.first().and_then(|v| v.as_number()).ok_or_else(|| {
        CorvoError::invalid_argument("time.format_local requires seconds (number)")
    })? as i64;

    let nsec = args
        .get(1)
        .and_then(|v| v.as_number())
        .map(|n| n.clamp(0.0, 1e9 - 1.0) as u32)
        .unwrap_or(0);

    let fmt = args
        .get(2)
        .and_then(|v| v.as_string())
        .ok_or_else(|| {
            CorvoError::invalid_argument(
                "time.format_local requires a format string as third argument (chrono strftime)",
            )
        })?
        .as_str();

    let dt = Local
        .timestamp_opt(secs, nsec)
        .single()
        .ok_or_else(|| CorvoError::invalid_argument("time.format_local: invalid timestamp"))?;

    Ok(Value::String(dt.format(fmt).to_string()))
}

/// Seconds since Unix epoch in local time interpretation for `format_local`.
pub fn unix_now(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "time.unix_now takes no arguments",
        ));
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| CorvoError::runtime(e.to_string()))?
        .as_secs_f64();
    Ok(Value::Number(secs))
}

/// Format a Unix timestamp in UTC timezone using a `strftime` pattern.
/// Args: `seconds: number`, `[nanoseconds: number]`, `format: string`
pub fn format_utc(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let secs =
        args.first().and_then(|v| v.as_number()).ok_or_else(|| {
            CorvoError::invalid_argument("time.format_utc requires seconds (number)")
        })? as i64;

    let nsec = args
        .get(1)
        .and_then(|v| v.as_number())
        .map(|n| n.clamp(0.0, 1e9 - 1.0) as u32)
        .unwrap_or(0);

    let fmt = args
        .get(2)
        .and_then(|v| v.as_string())
        .ok_or_else(|| {
            CorvoError::invalid_argument(
                "time.format_utc requires a format string as third argument (chrono strftime)",
            )
        })?
        .as_str();

    let dt = Utc
        .timestamp_opt(secs, nsec)
        .single()
        .ok_or_else(|| CorvoError::invalid_argument("time.format_utc: invalid timestamp"))?;

    Ok(Value::String(dt.format(fmt).to_string()))
}

/// Parse a date string and return the Unix timestamp (seconds).
/// Args: `date_string: string`, `[format: string]`
/// If format is not provided, common formats are tried automatically.
pub fn parse_date(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let date_str = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("time.parse_date requires a date string"))?;

    // Check for relative date strings
    let lower = date_str.to_lowercase();
    let now = Utc::now();

    // Handle "now"
    if lower == "now" {
        return Ok(Value::Number(now.timestamp() as f64));
    }

    // Handle "yesterday"
    if lower == "yesterday" {
        let yesterday = now - chrono::Duration::days(1);
        return Ok(Value::Number(yesterday.timestamp() as f64));
    }

    // Handle "tomorrow"
    if lower == "tomorrow" {
        let tomorrow = now + chrono::Duration::days(1);
        return Ok(Value::Number(tomorrow.timestamp() as f64));
    }

    // Handle "@timestamp" format (seconds since epoch)
    if let Some(ts_str) = date_str.strip_prefix('@') {
        let ts: i64 = ts_str.parse().map_err(|_| {
            CorvoError::invalid_argument(format!(
                "time.parse_date: invalid timestamp '{}'",
                date_str
            ))
        })?;
        return Ok(Value::Number(ts as f64));
    }

    // If explicit format provided, use it
    if let Some(fmt) = args.get(1).and_then(|v| v.as_string()) {
        // Try parsing with timezone
        if let Ok(dt) = DateTime::parse_from_str(date_str, fmt) {
            return Ok(Value::Number(dt.timestamp() as f64));
        }
        // Try parsing as naive (local time)
        if let Ok(ndt) = NaiveDateTime::parse_from_str(date_str, fmt) {
            let local = Local.from_local_datetime(&ndt).single().ok_or_else(|| {
                CorvoError::invalid_argument(format!(
                    "time.parse_date: ambiguous local time '{}'",
                    date_str
                ))
            })?;
            return Ok(Value::Number(local.timestamp() as f64));
        }
        return Err(CorvoError::invalid_argument(format!(
            "time.parse_date: cannot parse '{}' with format '{}'",
            date_str, fmt
        )));
    }

    // Try common formats
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d",
        "%Y/%m/%d %H:%M:%S",
        "%Y/%m/%d",
        "%d-%m-%Y %H:%M:%S",
        "%d-%m-%Y",
        "%m/%d/%Y %H:%M:%S",
        "%m/%d/%Y",
        "%b %d %Y %H:%M:%S",
        "%b %d, %Y %H:%M:%S",
        "%b %d %Y",
        "%b %d, %Y",
    ];

    for fmt in &formats {
        // Try parsing with timezone
        if let Ok(dt) = DateTime::parse_from_str(date_str, fmt) {
            return Ok(Value::Number(dt.timestamp() as f64));
        }
        // Try parsing as naive (assume local time)
        if let Ok(ndt) = NaiveDateTime::parse_from_str(date_str, fmt) {
            if let Some(local) = Local.from_local_datetime(&ndt).single() {
                return Ok(Value::Number(local.timestamp() as f64));
            }
        }
    }

    // Try RFC 2822 format
    if let Ok(dt) = DateTime::parse_from_rfc2822(date_str) {
        return Ok(Value::Number(dt.timestamp() as f64));
    }

    // Try RFC 3339 format
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Ok(Value::Number(dt.timestamp() as f64));
    }

    Err(CorvoError::invalid_argument(format!(
        "time.parse_date: cannot parse date string '{}'",
        date_str
    )))
}

/// Get the boot time as Unix timestamp (seconds since epoch).
/// Returns the time when the system was started.
pub fn boot_time(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    if !args.is_empty() {
        return Err(CorvoError::invalid_argument(
            "time.boot_time takes no arguments",
        ));
    }

    #[cfg(target_os = "linux")]
    {
        // Read /proc/uptime and compute boot time
        let uptime_secs = std::fs::read_to_string("/proc/uptime")
            .map_err(|e| CorvoError::io(format!("failed to read /proc/uptime: {}", e)))?
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| CorvoError::runtime("failed to parse /proc/uptime"))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| CorvoError::runtime(e.to_string()))?
            .as_secs_f64();

        Ok(Value::Number(now - uptime_secs))
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, run sysctl to get boot time
        let output = std::process::Command::new("sysctl")
            .args(["-n", "kern.boottime"])
            .output()
            .map_err(|e| CorvoError::io(format!("failed to run sysctl: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Output format: "{ sec = 1234567890, usec = 123456 } ..."
        let sec = stdout
            .split("sec = ")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.trim().parse::<f64>().ok())
            .ok_or_else(|| CorvoError::runtime("failed to parse sysctl boot time"))?;

        Ok(Value::Number(sec))
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use wmic or PowerShell to get boot time
        let output = std::process::Command::new("powershell")
            .args([
                "-Command",
                "(Get-CimInstance Win32_OperatingSystem).LastBootUpTime | Get-Date -UFormat %s",
            ])
            .output()
            .map_err(|e| CorvoError::io(format!("failed to get boot time: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let secs: f64 = stdout
            .trim()
            .parse()
            .map_err(|_| CorvoError::runtime("failed to parse boot time"))?;

        return Ok(Value::Number(secs));
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    Err(CorvoError::runtime(
        "time.boot_time is not supported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> HashMap<String, Value> {
        HashMap::new()
    }

    #[test]
    fn format_epoch_seconds() {
        let args = vec![
            Value::Number(0.0),
            Value::Number(0.0),
            Value::String("%s".to_string()),
        ];
        let s = format_local(&args, &empty()).unwrap();
        assert_eq!(s, Value::String("0".to_string()));
    }

    #[test]
    fn unix_now_positive() {
        let v = unix_now(&[], &empty()).unwrap();
        assert!(v.as_number().unwrap() > 1_600_000_000.0);
    }

    #[test]
    fn format_utc_epoch() {
        let args = vec![
            Value::Number(0.0),
            Value::Number(0.0),
            Value::String("%Y-%m-%d %H:%M:%S".to_string()),
        ];
        let s = format_utc(&args, &empty()).unwrap();
        assert_eq!(s, Value::String("1970-01-01 00:00:00".to_string()));
    }

    #[test]
    fn parse_date_now() {
        let args = vec![Value::String("now".to_string())];
        let v = parse_date(&args, &empty()).unwrap();
        let ts = v.as_number().unwrap();
        assert!(ts > 1_600_000_000.0);
    }

    #[test]
    fn parse_date_epoch() {
        let args = vec![Value::String("@0".to_string())];
        let v = parse_date(&args, &empty()).unwrap();
        assert_eq!(v, Value::Number(0.0));
    }

    #[test]
    fn parse_date_iso_format() {
        let args = vec![Value::String("1970-01-01 00:00:00".to_string())];
        let v = parse_date(&args, &empty()).unwrap();
        // The result depends on local timezone, but should be close to 0
        let ts = v.as_number().unwrap();
        // Allow for timezone offset (up to 14 hours = 50400 seconds)
        assert!(ts.abs() < 60000.0);
    }
}
