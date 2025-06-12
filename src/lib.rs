//! MCP Server as HTTP Core
//! 
//! This crate provides the core HTTP server functionality for exposing 
//! Model Context Protocol (MCP) servers via REST API.
//!
//! ## Features
//!
//! - **HTTP Server**: Axum-based REST API server
//! - **Authentication**: Bearer token validation middleware  
//! - **Runtime Abstraction**: Support for Node.js, Python, and Go MCP servers
//! - **Process Management**: Async stdin/stdout communication with MCP servers
//! - **Configuration**: Flexible JSON-based configuration system
//! - **Repository Management**: Automatic Git repository cloning and building
//!
//! ## Example
//!
//! ```rust,no_run
//! use mcp_server_as_http_core::http_server::McpHttpServer;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let server = McpHttpServer::new(
//!         "mcp_servers.config.json",
//!         "my-server", 
//!         "node"
//!     ).await?;
//!     
//!     server.serve(3000).await?;
//!     Ok(())
//! }
//! ```

pub mod auth;
pub mod config;
pub mod error;
pub mod http_server;
pub mod process;
pub mod runtime;

// Re-export commonly used types
pub use error::{McpCoreError, McpCoreResult};
pub use http_server::McpHttpServer;
pub use runtime::{McpRuntime, create_runtime};
pub use config::{McpServersConfig, McpServerConfig, AuthConfig, RuntimeConfig, NodeConfig, PythonConfig, GoConfig};
pub use process::{McpProcess, McpRequest, McpResponse};
pub use auth::{AuthError, bearer_auth_middleware};

/// Re-export commonly used external types
pub use serde::{Deserialize, Serialize};
pub use serde_json::{Value as JsonValue};
pub use tokio;
pub use axum;
