use thiserror::Error;
use crate::domain::error::DomainError; // ドメインエラーをラップするため
use crate::infrastructure::error::InfrastructureError; // InfrastructureError をラップするため

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("LGTM generation failed: {0}")]
    LgtmGenerationFailed(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Domain error occurred: {0}")]
    DomainError(#[from] DomainError), // ドメインエラーをラップ

    #[error("Infrastructure error occurred: {0}")]
    InfrastructureError(#[from] InfrastructureError), // InfrastructureError をラップ

    #[error("Underlying error: {source:?}")]
    AnyhowError {
        #[from]
        source: anyhow::Error,
    }
}

// IntoResponse implementation for ApplicationError
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;

impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApplicationError::LgtmGenerationFailed(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApplicationError::ExternalServiceError(msg) => (StatusCode::BAD_GATEWAY, msg),
            ApplicationError::ConfigurationError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApplicationError::DomainError(domain_err) => {
                (StatusCode::BAD_REQUEST, domain_err.to_string())
            }
            ApplicationError::InfrastructureError(infra_err) => {
                // Log the full infra_err for detailed debugging if possible
                eprintln!("InfrastructureError: {:?}", infra_err);
                // You might want to map specific infra errors to different status codes
                match infra_err {
                    InfrastructureError::ExternalApiError(_) => (StatusCode::BAD_GATEWAY, infra_err.to_string()),
                    InfrastructureError::DecodingError(_) => (StatusCode::BAD_REQUEST, infra_err.to_string()),
                    InfrastructureError::ImageLibError(_) => (StatusCode::UNPROCESSABLE_ENTITY, infra_err.to_string()),
                    _ => (StatusCode::INTERNAL_SERVER_ERROR, infra_err.to_string()),
                }
            }
            ApplicationError::AnyhowError{source} => {
                eprintln!("Unhandled AnyhowError: {:?}", source); // ログ出力
                (StatusCode::INTERNAL_SERVER_ERROR, "An unexpected error occurred.".to_string())
            }
        };
        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}
