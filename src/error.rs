//! Error types for MCP HTTP Core

use thiserror::Error;

/// Core error types for MCP HTTP operations
#[derive(Error, Debug)]
pub enum McpCoreError {
    #[error("Authentication failed: {message}")]
    AuthenticationError { message: String },

    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Process communication error: {message}")]
    ProcessError { message: String },

    #[error("Runtime error: {message}")]
    RuntimeError { message: String },

    #[error("HTTP server error: {message}")]
    HttpServerError { message: String },

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Convenient Result type for MCP Core operations
pub type McpCoreResult<T> = Result<T, McpCoreError>;
