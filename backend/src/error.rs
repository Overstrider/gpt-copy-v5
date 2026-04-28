use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::{config::ConfigError, openrouter::OpenRouterError};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("openrouter error: {0}")]
    OpenRouter(#[from] OpenRouterError),
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
}

impl AppError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound(resource.into())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Configuration(_) | Self::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::OpenRouter(error) => error.status_code(),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation_error",
            Self::NotFound(_) => "not_found",
            Self::Configuration(_) | Self::Config(_) => "configuration_error",
            Self::Database(_) => "database_error",
            Self::OpenRouter(_) => "openrouter_error",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let payload = ErrorResponse {
            error: ErrorBody {
                code: self.code(),
                message: self.to_string(),
            },
        };
        (status, Json(payload)).into_response()
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
}
