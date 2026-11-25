use std::path::PathBuf;
use zeroize::Zeroizing;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub password: Zeroizing<String>,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
    pub max_message_size: usize,
    pub rate_limit_per_second: u32,
    pub session_timeout_secs: u64,
}

impl ServerConfig {
    pub fn new(host: impl Into<String>, port: u16, password: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            password: Zeroizing::new(password.into()),
            cert_path: None,
            key_path: None,
            max_message_size: 65_536,   // 64 KB
            rate_limit_per_second: 10,  // 10 messages/sec burst
            session_timeout_secs: 3600, // 1 hour
        }
    }

    pub fn is_tls(&self) -> bool {
        self.cert_path.is_some() && self.key_path.is_some()
    }
}
