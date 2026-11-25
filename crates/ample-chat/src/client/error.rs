use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("WebSocket error: {0}")]
    WebSocket(Box<tokio_tungstenite::tungstenite::Error>),
    #[error("Protocol error: {0}")]
    Proto(#[from] ample_chat_proto::ProtocolError),
    #[error("Crypto error: {0}")]
    Crypto(#[from] ample_chat_proto::CryptoError),
    #[error("SRP error: {0}")]
    Srp(#[from] ample_chat_proto::SrpError),
    #[error("Codec error: {0}")]
    Codec(#[from] ample_chat_proto::CodecError),
    #[error("Server returned error ({status}): {message}")]
    ServerError { status: u16, message: String },
    #[error("Authentication failed: {0}")]
    Auth(String),
    #[error("URL parsing error: {0}")]
    Url(String),
}

impl From<tokio_tungstenite::tungstenite::Error> for ClientError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(err))
    }
}
