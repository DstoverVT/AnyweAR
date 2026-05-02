//! Error types and HTTP response conversion for the AnyweAR API.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use thiserror::Error;

/// Application error type converted into HTTP responses by the API layer.
#[derive(Debug, Error)]
pub enum AppError {
    /// The client sent an invalid request.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// A requested resource could not be found.
    #[error("not found: {0}")]
    NotFound(String),
    /// An unexpected server-side failure occurred.
    #[error("internal error: {0}")]
    Internal(String),
}

/// JSON error response body.
#[derive(Debug, Serialize)]
struct ErrorBody {
    /// Human-readable error message.
    error: String,
}

impl IntoResponse for AppError {
    /// Converts an application error into a status code and JSON body.
    ///
    /// # Arguments
    ///
    /// * `self` - The application error being rendered as an HTTP response.
    ///
    /// # Returns
    ///
    /// An Axum response containing an HTTP status code and JSON error body.
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = ErrorBody {
            error: self.to_string(),
        };

        (status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    /// Wraps a general-purpose error as an internal application error.
    ///
    /// # Arguments
    ///
    /// * `error` - The source error to expose as an internal error message.
    ///
    /// # Returns
    ///
    /// An [`AppError::Internal`] containing the source error text.
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error.to_string())
    }
}
