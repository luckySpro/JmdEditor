//! Errors returned to the UI as `{ code, message }`.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self { code: code.to_string(), message: message.into() }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid", message)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        let code = match e.kind() {
            std::io::ErrorKind::NotFound => "not_found",
            std::io::ErrorKind::PermissionDenied => "permission_denied",
            std::io::ErrorKind::StorageFull => "disk_full",
            _ => "io",
        };
        Self::new(code, e.to_string())
    }
}

impl From<jmd_import::ImportError> for AppError {
    fn from(e: jmd_import::ImportError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}

impl From<jmd_export::ExportError> for AppError {
    fn from(e: jmd_export::ExportError) -> Self {
        Self::new("export", e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::new("settings", e.to_string())
    }
}
