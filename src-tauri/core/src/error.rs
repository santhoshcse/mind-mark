use std::{error::Error, fmt};

#[derive(Debug)]
pub enum AppError {
    Database(rusqlite::Error),
    Io(std::io::Error),
    InvalidInput(String),
    Json(serde_json::Error),
    LockPoisoned,
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "database error: {error}"),
            Self::Io(error) => write!(formatter, "file access error: {error}"),
            Self::InvalidInput(message) => formatter.write_str(message),
            Self::Json(error) => write!(formatter, "invalid JSON: {error}"),
            Self::LockPoisoned => formatter.write_str("database lock is unavailable"),
        }
    }
}

impl Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
