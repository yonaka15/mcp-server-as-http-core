//! MCP Server as HTTP Core
//!
//! This crate provides the core HTTP server functionality for converting
//! Model Context Protocol (MCP) servers to REST API endpoints.

pub mod auth;
pub mod config;
pub mod error;
pub mod http_server;
pub mod process;

use crate::error::McpCoreResult;
use crate::http_server::McpHttpServer;
use std::env;
use tracing_subscriber;

#[tokio::main]
async fn main() -> McpCoreResult<()> {
    // Load environment variables from .env file if present
    // This will not override existing environment variables
    if let Err(e) = dotenvy::dotenv() {
        // It's okay if .env file doesn't exist
        tracing::debug!("No .env file found or error loading it: {}", e);
    }

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mcp_server_as_http_core=debug".parse().unwrap()),
        )
        .init();

    tracing::info!("Starting MCP HTTP Core server...");

    // Get configuration from environment variables
    let config_file =
        env::var("MCP_CONFIG_FILE").unwrap_or_else(|_| "mcp_servers.config.json".to_string());
    let server_name = env::var("MCP_SERVER_NAME").unwrap_or_else(|_| "redmine".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);

    tracing::info!(
        "Configuration - Config: {}, Server: {}, Port: {}",
        config_file,
        server_name,
        port
    );

    // Create and start the MCP HTTP server
    let server = McpHttpServer::new(&config_file, &server_name).await?;

    tracing::info!("MCP HTTP Core server ready to accept connections");

    // Start serving
    server.serve(port).await?;

    Ok(())
}
