use std::io;

/// Custom error type for SiloFS operations
#[derive(Debug)]
pub enum SiloFSError {
    Io(io::Error),
    SerdeJson(serde_json::Error),
}

impl From<io::Error> for SiloFSError {
    fn from(error: io::Error) -> Self {
        SiloFSError::Io(error)
    }
}

impl From<serde_json::Error> for SiloFSError {
    fn from(error: serde_json::Error) -> Self {
        SiloFSError::SerdeJson(error)
    }
}

impl std::fmt::Display for SiloFSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SiloFSError::Io(e) => write!(f, "I/O error: {}", e),
            SiloFSError::SerdeJson(e) => write!(f, "Serde JSON error: {}", e),
        }
    }
}

impl std::error::Error for SiloFSError {}