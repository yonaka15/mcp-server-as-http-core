//! Authentication module for MCP HTTP Core

use crate::config::AuthConfig;
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Authentication error response
#[derive(Serialize)]
pub struct AuthError {
    pub error: String,
    pub message: String,
}

/// Bearer token authentication middleware
pub async fn bearer_auth_middleware(
    State(auth_config): State<AuthConfig>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // Skip authentication if disabled
    if !auth_config.enabled {
        tracing::debug!("Authentication disabled, proceeding without check");
        return Ok(next.run(request).await);
    }

    // Skip if no API key is configured
    let expected_api_key = match &auth_config.api_key {
        Some(key) => key,
        None => {
            tracing::debug!("No API key configured, proceeding without check");
            return Ok(next.run(request).await);
        }
    };

    // Extract Authorization header
    let auth_header = match headers.get("authorization") {
        Some(header) => match header.to_str() {
            Ok(header_str) => header_str,
            Err(_) => {
                tracing::debug!("Invalid Authorization header format");
                let error_response = AuthError {
                    error: "Unauthorized".to_string(),
                    message: "Invalid Authorization header format".to_string(),
                };
                return Err((StatusCode::UNAUTHORIZED, Json(error_response)));
            }
        },
        None => {
            tracing::debug!("Missing Authorization header");
            let error_response = AuthError {
                error: "Unauthorized".to_string(),
                message: "Missing Authorization header".to_string(),
            };
            return Err((StatusCode::UNAUTHORIZED, Json(error_response)));
        }
    };

    // Extract Bearer token
    if !auth_header.starts_with("Bearer ") {
        tracing::debug!("Authorization header does not start with 'Bearer '");
        let error_response = AuthError {
            error: "Unauthorized".to_string(),
            message: "Authorization header must use Bearer token".to_string(),
        };
        return Err((StatusCode::UNAUTHORIZED, Json(error_response)));
    }

    let provided_token = &auth_header[7..]; // Skip "Bearer "

    // Validate API key
    if provided_token != expected_api_key {
        tracing::debug!(
            "Invalid API key provided (length: {})",
            provided_token.len()
        );
        let error_response = AuthError {
            error: "Unauthorized".to_string(),
            message: "Invalid API key".to_string(),
        };
        return Err((StatusCode::UNAUTHORIZED, Json(error_response)));
    }

    tracing::debug!("Authentication successful");
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use std::collections::HashMap;

    #[test]
    fn test_auth_error_serialization() {
        let error = AuthError {
            error: "Unauthorized".to_string(),
            message: "Test message".to_string(),
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Unauthorized"));
        assert!(json.contains("Test message"));
    }
}
