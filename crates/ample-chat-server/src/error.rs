use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Protocol error: {0}")]
    Proto(#[from] ample_chat_proto::ProtocolError),
    #[error("SRP error: {0}")]
    Srp(#[from] ample_chat_proto::SrpError),
    #[error("TLS configuration error: {0}")]
    Tls(String),
    #[error("Server configuration error: {0}")]
    Config(String),
}
