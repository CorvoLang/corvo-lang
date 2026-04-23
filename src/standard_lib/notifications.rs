use crate::type_system::Value;
use crate::{CorvoError, CorvoResult};
use std::collections::HashMap;

/// Current Facebook Graph API version used by `notifications.messenger`.
const FB_API_VERSION: &str = "v18.0";

// ---------------------------------------------------------------------------
// OAuth 1.0a helper — used by `notifications.x`
// ---------------------------------------------------------------------------

/// Percent-encode a string according to RFC 3986 (used for OAuth 1.0a signatures).
fn percent_encode(s: &str) -> String {
    let mut encoded = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}

// ---------------------------------------------------------------------------
// SMTP
// ---------------------------------------------------------------------------

/// Send an email via SMTP.
///
/// Args: host, port, username, password, from_addr, to_addr, subject, body
/// Returns: map { success: bool }
pub fn smtp(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    use lettre::message::header::ContentType;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};

    let host = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("notifications.smtp requires host as arg 1"))?;

    let port =
        args.get(1).and_then(|v| v.as_number()).ok_or_else(|| {
            CorvoError::invalid_argument("notifications.smtp requires port as arg 2")
        })? as u16;

    let username = args
        .get(2)
        .and_then(|v| v.as_string())
        .cloned()
        .ok_or_else(|| {
            CorvoError::invalid_argument("notifications.smtp requires username as arg 3")
        })?;

    let password = args
        .get(3)
        .and_then(|v| v.as_string())
        .cloned()
        .ok_or_else(|| {
            CorvoError::invalid_argument("notifications.smtp requires password as arg 4")
        })?;

    let from_addr = args
        .get(4)
        .and_then(|v| v.as_string())
        .cloned()
        .ok_or_else(|| {
            CorvoError::invalid_argument("notifications.smtp requires from_addr as arg 5")
        })?;

    let to_addr = args
        .get(5)
        .and_then(|v| v.as_string())
        .cloned()
        .ok_or_else(|| {
            CorvoError::invalid_argument("notifications.smtp requires to_addr as arg 6")
        })?;

    let subject = args
        .get(6)
        .and_then(|v| v.as_string())
        .cloned()
        .ok_or_else(|| {
            CorvoError::invalid_argument("notifications.smtp requires subject as arg 7")
        })?;

    let body = args
        .get(7)
        .and_then(|v| v.as_string())
        .cloned()
        .ok_or_else(|| CorvoError::invalid_argument("notifications.smtp requires body as arg 8"))?;

    let from_mailbox: lettre::message::Mailbox = from_addr
        .parse()
        .map_err(|e: lettre::address::AddressError| CorvoError::invalid_argument(e.to_string()))?;

    let to_mailbox: lettre::message::Mailbox = to_addr
        .parse()
        .map_err(|e: lettre::address::AddressError| CorvoError::invalid_argument(e.to_string()))?;

    let email = Message::builder()
        .from(from_mailbox)
        .to(to_mailbox)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .map_err(|e| CorvoError::runtime(e.to_string()))?;

    let creds = Credentials::new(username, password);

    let mailer = SmtpTransport::starttls_relay(host)
        .map_err(|e| CorvoError::network(e.to_string()))?
        .credentials(creds)
        .port(port)
        .build();

    mailer
        .send(&email)
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert("success".to_string(), Value::Boolean(true));
    Ok(Value::Map(result))
}

// ---------------------------------------------------------------------------
// Slack
// ---------------------------------------------------------------------------

/// Post a message to a Slack incoming webhook.
///
/// Args: webhook_url, message
/// Returns: map { status_code: number, response_body: string }
pub fn slack(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let webhook_url = args.first().and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.slack requires webhook_url as arg 1")
    })?;

    let message = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.slack requires message as arg 2")
    })?;

    let payload = serde_json::json!({ "text": message });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );
    Ok(Value::Map(result))
}

// ---------------------------------------------------------------------------
// Telegram
// ---------------------------------------------------------------------------

/// Send a message via Telegram Bot API.
///
/// Args: bot_token, chat_id, message
/// Returns: map { status_code: number, response_body: string }
pub fn telegram(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let bot_token = args.first().and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.telegram requires bot_token as arg 1")
    })?;

    let chat_id = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.telegram requires chat_id as arg 2")
    })?;

    let message = args.get(2).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.telegram requires message as arg 3")
    })?;

    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let payload = serde_json::json!({
        "chat_id": chat_id,
        "text": message
    });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );
    Ok(Value::Map(result))
}

// ---------------------------------------------------------------------------
// Mattermost
// ---------------------------------------------------------------------------

/// Post a message to a Mattermost incoming webhook.
///
/// Args: webhook_url, message
/// Returns: map { status_code: number, response_body: string }
pub fn mattermost(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let webhook_url = args.first().and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.mattermost requires webhook_url as arg 1")
    })?;

    let message = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.mattermost requires message as arg 2")
    })?;

    let payload = serde_json::json!({ "text": message });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );
    Ok(Value::Map(result))
}

// ---------------------------------------------------------------------------
// Gitter
// ---------------------------------------------------------------------------

/// Post a message to a Gitter room.
///
/// Args: token, room_id, message
/// Returns: map { status_code: number, response_body: string }
pub fn gitter(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let token = args.first().and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.gitter requires token as arg 1")
    })?;

    let room_id = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.gitter requires room_id as arg 2")
    })?;

    let message = args.get(2).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.gitter requires message as arg 3")
    })?;

    let url = format!("https://api.gitter.im/v1/rooms/{}/chatMessages", room_id);
    let payload = serde_json::json!({ "text": message });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );
    Ok(Value::Map(result))
}

// ---------------------------------------------------------------------------
// Messenger (Facebook Messenger)
// ---------------------------------------------------------------------------

/// Send a message via the Facebook Messenger Send API.
///
/// Args: page_access_token, recipient_id, message
/// Returns: map { status_code: number, response_body: string }
pub fn messenger(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let page_access_token = args.first().and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.messenger requires page_access_token as arg 1")
    })?;

    let recipient_id = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.messenger requires recipient_id as arg 2")
    })?;

    let message = args.get(2).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.messenger requires message as arg 3")
    })?;

    let url = format!(
        "https://graph.facebook.com/{}/me/messages?access_token={}",
        FB_API_VERSION, page_access_token
    );
    let payload = serde_json::json!({
        "recipient": { "id": recipient_id },
        "message": { "text": message }
    });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );
    Ok(Value::Map(result))
}

// ---------------------------------------------------------------------------
// Discord
// ---------------------------------------------------------------------------

/// Post a message to a Discord webhook.
///
/// Args: webhook_url, message
/// Returns: map { status_code: number, response_body: string }
pub fn discord(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let webhook_url = args.first().and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.discord requires webhook_url as arg 1")
    })?;

    let message = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.discord requires message as arg 2")
    })?;

    let payload = serde_json::json!({ "content": message });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );
    Ok(Value::Map(result))
}

// ---------------------------------------------------------------------------
// Microsoft Teams
// ---------------------------------------------------------------------------

/// Post a message to a Microsoft Teams incoming webhook.
///
/// Args: webhook_url, message
/// Returns: map { status_code: number, response_body: string }
pub fn teams(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let webhook_url = args.first().and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.teams requires webhook_url as arg 1")
    })?;

    let message = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.teams requires message as arg 2")
    })?;

    // Teams webhooks accept the legacy MessageCard format for broad compatibility.
    let payload = serde_json::json!({
        "@type": "MessageCard",
        "@context": "https://schema.org/extensions",
        "text": message
    });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(webhook_url)
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );
    Ok(Value::Map(result))
}

// ---------------------------------------------------------------------------
// X (Twitter)
// ---------------------------------------------------------------------------

/// Post a tweet via the Twitter API v2 using OAuth 1.0a.
///
/// Args: api_key, api_secret, access_token, access_token_secret, message
/// Returns: map { status_code: number, response_body: string }
pub fn x(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use hmac::{Hmac, Mac};
    use sha1::Sha1;

    let api_key = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("notifications.x requires api_key as arg 1"))?;

    let api_secret = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.x requires api_secret as arg 2")
    })?;

    let access_token = args.get(2).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.x requires access_token as arg 3")
    })?;

    let access_token_secret = args.get(3).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.x requires access_token_secret as arg 4")
    })?;

    let message = args
        .get(4)
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("notifications.x requires message as arg 5"))?;

    let url = "https://api.twitter.com/2/tweets";
    let method = "POST";

    // Build OAuth 1.0a parameters
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let nonce = uuid::Uuid::new_v4().to_string().replace('-', "");

    let mut oauth_params: Vec<(&str, String)> = vec![
        ("oauth_consumer_key", api_key.clone()),
        ("oauth_nonce", nonce.clone()),
        ("oauth_signature_method", "HMAC-SHA1".to_string()),
        ("oauth_timestamp", timestamp.clone()),
        ("oauth_token", access_token.clone()),
        ("oauth_version", "1.0".to_string()),
    ];

    // Sort params alphabetically and build the parameter string
    oauth_params.sort_by(|a, b| a.0.cmp(b.0));
    let param_string = oauth_params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    // Build signature base string
    let base_string = format!(
        "{}&{}&{}",
        percent_encode(method),
        percent_encode(url),
        percent_encode(&param_string)
    );

    // Build signing key
    let signing_key = format!(
        "{}&{}",
        percent_encode(api_secret),
        percent_encode(access_token_secret)
    );

    // Compute HMAC-SHA1
    let mut mac = Hmac::<Sha1>::new_from_slice(signing_key.as_bytes())
        .map_err(|e| CorvoError::runtime(e.to_string()))?;
    mac.update(base_string.as_bytes());
    let signature = BASE64.encode(mac.finalize().into_bytes());

    // Build the Authorization header
    let auth_header = format!(
        "OAuth oauth_consumer_key=\"{}\", oauth_nonce=\"{}\", oauth_signature=\"{}\", \
         oauth_signature_method=\"HMAC-SHA1\", oauth_timestamp=\"{}\", \
         oauth_token=\"{}\", oauth_version=\"1.0\"",
        percent_encode(api_key),
        percent_encode(&nonce),
        percent_encode(&signature),
        percent_encode(&timestamp),
        percent_encode(access_token),
    );

    let body_payload = serde_json::json!({ "text": message });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url)
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .body(body_payload.to_string())
        .send()
        .map_err(|e| CorvoError::network(e.to_string()))?;

    let mut result = HashMap::new();
    result.insert(
        "status_code".to_string(),
        Value::Number(response.status().as_u16() as f64),
    );
    result.insert(
        "response_body".to_string(),
        Value::String(response.text().unwrap_or_default()),
    );
    Ok(Value::Map(result))
}

// ---------------------------------------------------------------------------
// Local OS Notification
// ---------------------------------------------------------------------------

/// Show a local desktop notification using platform-specific tools.
///
/// Args: title, message
/// Returns: map { success: bool }
pub fn os_notify(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    let title = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("notifications.os requires title as arg 1"))?;

    let message = args.get(1).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.os requires message as arg 2")
    })?;

    let success = send_os_notification(title, message);

    let mut result = HashMap::new();
    result.insert("success".to_string(), Value::Boolean(success));
    Ok(Value::Map(result))
}

fn send_os_notification(title: &str, message: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("notify-send")
            .arg(title)
            .arg(message)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            message.replace('"', "\\\""),
            title.replace('"', "\\\"")
        );
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, \
             ContentType = WindowsRuntime] > $null; \
             $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent(\
             [Windows.UI.Notifications.ToastTemplateType]::ToastText02); \
             $template.SelectSingleNode('//text[@id=1]').InnerText = '{}'; \
             $template.SelectSingleNode('//text[@id=2]').InnerText = '{}'; \
             $toast = [Windows.UI.Notifications.ToastNotification]::new($template); \
             [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Corvo').Show($toast)",
            title.replace('\'', "\\'"),
            message.replace('\'', "\\'")
        );
        std::process::Command::new("powershell")
            .args(["-Command", &script])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        // Fallback: unsupported platform
        let _ = (title, message);
        false
    }
}

// ---------------------------------------------------------------------------
// IRC
// ---------------------------------------------------------------------------

/// Send a message to an IRC channel.
///
/// Connects to `host:port` over plain TCP, authenticates with an optional
/// `password` (PASS command, pass `""` to skip), identifies as `nickname`,
/// joins `channel`, sends the `message` as a PRIVMSG, and disconnects.
///
/// Args: host, port, nickname, channel, message, \[password\]
/// Returns: map { success: bool }
pub fn irc(args: &[Value], _named_args: &HashMap<String, Value>) -> CorvoResult<Value> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let host = args
        .first()
        .and_then(|v| v.as_string())
        .ok_or_else(|| CorvoError::invalid_argument("notifications.irc requires host as arg 1"))?;

    let port =
        args.get(1).and_then(|v| v.as_number()).ok_or_else(|| {
            CorvoError::invalid_argument("notifications.irc requires port as arg 2")
        })? as u16;

    let nickname = args.get(2).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.irc requires nickname as arg 3")
    })?;

    let channel = args.get(3).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.irc requires channel as arg 4")
    })?;

    let message = args.get(4).and_then(|v| v.as_string()).ok_or_else(|| {
        CorvoError::invalid_argument("notifications.irc requires message as arg 5")
    })?;

    // Optional server password (arg 6); empty string means no PASS command.
    let password = args
        .get(5)
        .and_then(|v| v.as_string())
        .cloned()
        .unwrap_or_default();

    let addr = format!("{}:{}", host, port);
    // Resolve the address first so we can use connect_timeout (which requires
    // a concrete SocketAddr rather than a hostname string).
    let sock_addr = addr.parse::<std::net::SocketAddr>().or_else(|_| {
        use std::net::ToSocketAddrs;
        addr.to_socket_addrs()
            .map_err(|e| CorvoError::network(e.to_string()))?
            .next()
            .ok_or_else(|| CorvoError::network(format!("could not resolve {}", addr)))
    })?;
    let stream = TcpStream::connect_timeout(&sock_addr, Duration::from_secs(10))
        .map_err(|e| CorvoError::network(e.to_string()))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| CorvoError::io(e.to_string()))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| CorvoError::io(e.to_string()))?;

    let mut writer = stream
        .try_clone()
        .map_err(|e| CorvoError::io(e.to_string()))?;
    let reader = BufReader::new(stream);

    // Helper: send a single IRC line (adds \r\n).
    let send = |w: &mut dyn Write, line: &str| -> CorvoResult<()> {
        w.write_all(format!("{}\r\n", line).as_bytes())
            .map_err(|e| CorvoError::io(e.to_string()))
    };

    // Authenticate / register.
    if !password.is_empty() {
        send(&mut writer, &format!("PASS {}", password))?;
    }
    send(&mut writer, &format!("NICK {}", nickname))?;
    send(&mut writer, &format!("USER {} 0 * :{}", nickname, nickname))?;

    // Wait for the 001 (RPL_WELCOME) numeric to confirm registration,
    // handling PING challenges along the way.
    for line in reader.lines() {
        let line = line.map_err(|e| CorvoError::io(e.to_string()))?;

        // Respond to PING to keep the connection alive during registration.
        if line.starts_with("PING") {
            let token = line.trim_start_matches("PING").trim();
            send(&mut writer, &format!("PONG {}", token))?;
        }

        // 001 == RPL_WELCOME — we are registered.
        if line.split_whitespace().nth(1) == Some("001") {
            break;
        }

        // 4xx / 5xx error numerics abort registration.
        if let Some(code) = line.split_whitespace().nth(1) {
            if code.starts_with('4') || code.starts_with('5') {
                return Err(CorvoError::network(format!(
                    "IRC server rejected registration: {}",
                    line
                )));
            }
        }
    }

    // Ensure channel starts with '#'.
    let channel_name: std::borrow::Cow<str> = if channel.starts_with('#') {
        std::borrow::Cow::Borrowed(channel.as_str())
    } else {
        std::borrow::Cow::Owned(format!("#{}", channel))
    };

    send(&mut writer, &format!("JOIN {}", channel_name))?;
    send(
        &mut writer,
        &format!("PRIVMSG {} :{}", channel_name, message),
    )?;
    send(&mut writer, "QUIT :done")?;

    let mut result = HashMap::new();
    result.insert("success".to_string(), Value::Boolean(true));
    Ok(Value::Map(result))
}
