use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    debug: String,
    user: String,
    kind: ErrorKind,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ErrorKind {
    FS,
    Serialization,
    Tauri,
    Request,
}

impl Error {
    pub fn new(kind: ErrorKind, debug: String, user: &str) -> Self {
        Self {
            kind,
            debug,
            user: user.to_string(),
        }
    }
}

impl From<tauri::Error> for Error {
    fn from(value: tauri::Error) -> Self {
        Self {
            debug: value.to_string(),
            user: value.to_string(),
            kind: ErrorKind::Tauri,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for Error {}
