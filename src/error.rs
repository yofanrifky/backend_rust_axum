use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::fmt::Display;

/// Error API dengan status HTTP dan pesan yang aman untuk dikirim ke klien.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }

    /// Catat rincian teknis di log dan kirim pesan umum yang aman ke klien.
    pub fn internal(message: &str, source: impl Display) -> Self {
        eprintln!("[ERROR] {message}: {source}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn internal_message(message: &str) -> Self {
        eprintln!("[ERROR] {message}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body: Value = json!({ "error": self.message });
        (self.status, Json(body)).into_response()
    }
}
