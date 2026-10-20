//! Token-protected HTTP webhooks for Wi-Fi-capable builds.
//!
//! This is intentionally a small, plain-HTTP LAN endpoint. For internet-origin
//! services, expose it through an authenticated tunnel or a relay that validates
//! the original sender before forwarding here.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, TryRecvError},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

const MAX_HEADER_BYTES: usize = 8 * 1024;
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
const MAX_AUDIO_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    #[serde(default)]
    pub id: String,
    pub title: String,
    pub body: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_wav_base64: Option<String>,
}

fn default_source() -> String {
    "webhook".into()
}

fn default_kind() -> String {
    "notice".into()
}

impl Notice {
    fn validate(&self) -> Result<(), &'static str> {
        if self.title.trim().is_empty() || self.title.chars().count() > 64 {
            return Err("title must contain 1 to 64 characters");
        }
        if self.body.chars().count() > 180 {
            return Err("body must not exceed 180 characters");
        }
        if self.id.chars().count() > 96
            || self.source.chars().count() > 32
            || self.kind.chars().count() > 24
        {
            return Err("id, source, or kind is too long");
        }
        if [&self.title, &self.body, &self.id, &self.source, &self.kind]
            .into_iter()
            .any(|value| value.chars().any(char::is_control))
        {
            return Err("notification fields cannot contain control characters");
        }
        if self.audio_wav_base64.is_some() {
            self.decoded_audio()?;
        }
        Ok(())
    }

    pub fn decoded_audio(&self) -> Result<Option<Vec<u8>>, &'static str> {
        let Some(encoded) = &self.audio_wav_base64 else {
            return Ok(None);
        };
        if encoded.len() > MAX_AUDIO_BYTES * 4 / 3 + 8 {
            return Err("audio payload is too large");
        }
        let bytes = BASE64
            .decode(encoded)
            .map_err(|_| "audio must be valid base64")?;
        if bytes.len() > MAX_AUDIO_BYTES {
            return Err("decoded audio is too large");
        }
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err("audio must be a WAV file");
        }
        Ok(Some(bytes))
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookConfig {
    pub bind: String,
    pub token: String,
}

impl WebhookConfig {
    pub fn load(path: &Path) -> Result<Self, String> {
        let metadata = fs::metadata(path)
            .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
        if !metadata.is_file() || metadata.len() > 4096 {
            return Err("webhook config must be a regular file under 4 KiB".into());
        }
        let bytes =
            fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let config: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid webhook config: {error}"))?;
        config
            .bind
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid bind address: {error}"))?;
        if !(16..=256).contains(&config.token.len()) {
            return Err("webhook token must contain 16 to 256 bytes".into());
        }
        Ok(config)
    }
}

pub struct WebhookServer {
    receiver: Receiver<Notice>,
    running: Arc<AtomicBool>,
    address: SocketAddr,
    thread: Option<JoinHandle<()>>,
}

impl WebhookServer {
    pub fn start(bind: &str, token: &str) -> io::Result<Self> {
        if token.len() < 16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "webhook token must contain at least 16 bytes",
            ));
        }
        let listener = TcpListener::bind(bind)?;
        let address = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        let (sender, receiver) = mpsc::channel();
        let running = Arc::new(AtomicBool::new(true));
        let thread_running = Arc::clone(&running);
        let token = Arc::new(token.as_bytes().to_vec());
        let thread = thread::spawn(move || {
            while thread_running.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        if let Ok(notice) = serve(&mut stream, &token) {
                            let _ = sender.send(notice);
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(25));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            receiver,
            running,
            address,
            thread: Some(thread),
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn try_recv(&self) -> Result<Notice, TryRecvError> {
        self.receiver.try_recv()
    }
}

impl Drop for WebhookServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(stream: &mut TcpStream, token: &[u8]) -> io::Result<Notice> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut request = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        if let Some(position) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
            break position + 4;
        }
        if request.len() >= MAX_HEADER_BYTES {
            return respond(stream, 431, "headers too large");
        }
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return respond(stream, 400, "incomplete request");
        }
        request.extend_from_slice(&chunk[..count]);
    };
    let head = std::str::from_utf8(&request[..header_end - 4])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "HTTP headers are not UTF-8"))?;
    let mut lines = head.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request line"))?;
    if request_line != "POST /webhook HTTP/1.1" {
        return respond(stream, 404, "use POST /webhook");
    }
    let mut content_length = 0_usize;
    let mut authorized = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            let Ok(length) = value.parse::<usize>() else {
                return respond(stream, 400, "invalid content-length");
            };
            content_length = length;
        } else if name.eq_ignore_ascii_case("authorization") {
            if let Some(candidate) = value.strip_prefix("Bearer ") {
                authorized = constant_time_eq(candidate.as_bytes(), token);
            }
        } else if name.eq_ignore_ascii_case("x-muku-token") {
            authorized = constant_time_eq(value.as_bytes(), token);
        }
    }
    if !authorized {
        return respond(stream, 401, "unauthorized");
    }
    if content_length == 0 || content_length > MAX_BODY_BYTES {
        return respond(stream, 413, "body must be 1 byte to 4 MiB");
    }
    while request.len() < header_end + content_length {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return respond(stream, 400, "incomplete body");
        }
        request.extend_from_slice(&chunk[..count]);
        if request.len() > header_end + MAX_BODY_BYTES {
            return respond(stream, 413, "body too large");
        }
    }
    let notice: Notice =
        match serde_json::from_slice(&request[header_end..header_end + content_length]) {
            Ok(notice) => notice,
            Err(_) => return respond(stream, 422, "invalid notification JSON"),
        };
    if let Err(message) = notice.validate() {
        return respond(stream, 422, message);
    }
    write_response(stream, 202, "accepted")?;
    Ok(notice)
}

fn respond(stream: &mut TcpStream, status: u16, message: &str) -> io::Result<Notice> {
    write_response(stream, status, message)?;
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "HTTP rejection response sent",
    ))
}

fn write_response(stream: &mut TcpStream, status: u16, message: &str) -> io::Result<()> {
    let reason = match status {
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        422 => "Unprocessable Content",
        431 => "Request Header Fields Too Large",
        _ => "Error",
    };
    let body =
        serde_json::json!({ "ok": (200..300).contains(&status), "message": message }).to_string();
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        difference |= usize::from(
            left.get(index).copied().unwrap_or(0) ^ right.get(index).copied().unwrap_or(0),
        );
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notice_validation_rejects_controls_and_oversize() {
        let mut notice = Notice {
            id: "test".into(),
            title: "hello".into(),
            body: "from ai".into(),
            source: "ai".into(),
            kind: "notice".into(),
            audio_wav_base64: None,
        };
        assert!(notice.validate().is_ok());
        notice.body = "line\nbreak".into();
        assert!(notice.validate().is_err());
        notice.body = "a".repeat(181);
        assert!(notice.validate().is_err());
        notice.body = "from ai".into();
        notice.audio_wav_base64 = Some(BASE64.encode(b"not a wav file"));
        assert!(notice.validate().is_err());
        notice.audio_wav_base64 = Some(BASE64.encode(b"RIFF\x24\x00\x00\x00WAVEfmt "));
        assert!(notice.validate().is_ok());
    }

    #[test]
    fn config_requires_bind_address_and_strong_token() {
        let directory =
            std::env::temp_dir().join(format!("muku-webhook-config-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("webhook.json");
        fs::write(
            &path,
            br#"{"bind":"127.0.0.1:8787","token":"0123456789abcdef"}"#,
        )
        .unwrap();
        assert_eq!(WebhookConfig::load(&path).unwrap().bind, "127.0.0.1:8787");
        fs::write(&path, br#"{"bind":"bad","token":"short"}"#).unwrap();
        assert!(WebhookConfig::load(&path).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn tokens_compare_without_early_exit() {
        assert!(constant_time_eq(b"same-long-token", b"same-long-token"));
        assert!(!constant_time_eq(b"same-long-token", b"different-token"));
        assert!(!constant_time_eq(b"short", b"longer"));
    }

    #[test]
    fn server_receives_authorized_notification() {
        let token = "0123456789abcdef";
        let server = WebhookServer::start("127.0.0.1:0", token).unwrap();
        let mut client = TcpStream::connect(server.address()).unwrap();
        let wav = b"RIFF\x24\x00\x00\x00WAVEfmt ";
        let body = serde_json::json!({
            "title": "AI says hello",
            "body": "time for a break",
            "source": "ai",
            "audio_wav_base64": BASE64.encode(wav),
        })
        .to_string();
        write!(
            client,
            "POST /webhook HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 202"));
        let notice = (0..100)
            .find_map(|_| {
                server.try_recv().ok().or_else(|| {
                    thread::sleep(Duration::from_millis(1));
                    None
                })
            })
            .unwrap();
        assert_eq!(notice.title, "AI says hello");
        assert_eq!(notice.source, "ai");
        assert_eq!(notice.decoded_audio().unwrap().unwrap(), wav);
    }

    #[test]
    fn server_rejects_wrong_token() {
        let server = WebhookServer::start("127.0.0.1:0", "0123456789abcdef").unwrap();
        let mut client = TcpStream::connect(server.address()).unwrap();
        let body = r#"{"title":"no","body":"access","source":"test"}"#;
        write!(
            client,
            "POST /webhook HTTP/1.1\r\nAuthorization: Bearer wrong-token\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 401"));
        assert!(server.try_recv().is_err());
    }
}
