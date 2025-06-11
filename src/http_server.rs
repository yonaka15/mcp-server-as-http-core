//! HTTP server module for MCP Core

use axum::{extract::State, http::StatusCode, middleware, response::Json, routing::post, Router};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing;

use crate::{
    auth::bearer_auth_middleware,
    config::{AuthConfig, McpServersConfig},
    error::{McpCoreError, McpCoreResult},
    process::{McpProcess, McpRequest, McpResponse},
    runtime::{create_runtime},
};

/// HTTP server state containing the MCP process
#[derive(Clone)]
pub struct ServerState {
    pub mcp_process: Arc<Mutex<McpProcess>>,
}

/// HTTP server for MCP Core
pub struct McpHttpServer {
    auth_config: AuthConfig,
    server_state: ServerState,
}

impl McpHttpServer {
    /// Create a new MCP HTTP server
    pub async fn new(
        config_file_path: &str,
        server_name: &str,
        runtime_type: &str,
    ) -> McpCoreResult<Self> {
        tracing::info!("Initializing MCP HTTP server...");
        tracing::info!(
            "Config file: '{}', Server: '{}', Runtime: '{}'",
            config_file_path,
            server_name,
            runtime_type
        );

        // Load configuration
        let servers_config = McpServersConfig::load_from_file(config_file_path).await?;
        let server_config = servers_config.get_server(server_name)?.clone();

        // Create runtime
        let runtime = create_runtime(runtime_type)?;

        // Setup environment
        runtime
            .setup_environment(&server_config.runtime_config)
            .await?;

        // Setup repository and get working directory
        let work_dir = "/tmp/mcp-servers";
        tokio::fs::create_dir_all(work_dir)
            .await
            .map_err(|e| McpCoreError::RuntimeError {
                message: format!("Failed to create work directory: {}", e),
            })?;

        let working_dir = runtime.setup_repository(&server_config, work_dir).await?;

        // Start MCP server process
        let mcp_process = runtime.start_server(&server_config, &working_dir).await?;

        // Create auth config
        let auth_config = AuthConfig::from_env();

        tracing::info!("MCP HTTP server initialized successfully");

        Ok(Self {
            auth_config,
            server_state: ServerState {
                mcp_process: Arc::new(Mutex::new(mcp_process)),
            },
        })
    }

    /// Create the Axum router
    pub fn create_router(self) -> Router {
        Router::new()
            .route("/api/v1", post(handle_mcp_request))
            .layer(middleware::from_fn_with_state(
                self.auth_config.clone(),
                bearer_auth_middleware,
            ))
            .with_state(self.server_state)
    }

    /// Start the HTTP server
    pub async fn serve(self, port: u16) -> McpCoreResult<()> {
        let app = self.create_router();

        let listener_addr = format!("0.0.0.0:{}", port);
        tracing::info!("Starting HTTP server on {}", listener_addr);

        let listener = tokio::net::TcpListener::bind(&listener_addr)
            .await
            .map_err(|e| McpCoreError::HttpServerError {
                message: format!("Failed to bind to address {}: {}", listener_addr, e),
            })?;

        tracing::info!(
            "HTTP server listening on http://{}",
            listener
                .local_addr()
                .map_err(|e| McpCoreError::HttpServerError {
                    message: format!("Failed to get local address: {}", e),
                })?
        );

        axum::serve(listener, app.into_make_service())
            .await
            .map_err(|e| McpCoreError::HttpServerError {
                message: format!("Server error: {}", e),
            })?;

        Ok(())
    }
}

/// Handle MCP requests via HTTP
async fn handle_mcp_request(
    State(server_state): State<ServerState>,
    Json(payload): Json<McpRequest>,
) -> Result<Json<McpResponse>, StatusCode> {
    tracing::debug!("Received HTTP request: {:?}", payload);

    let mut mcp_process_guard = server_state.mcp_process.lock().await;
    tracing::debug!("Acquired MCP process mutex lock");

    match mcp_process_guard.query(&payload).await {
        Ok(response) => {
            tracing::debug!("MCP query successful: {:?}", response);
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("MCP query failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create a simple health check endpoint
pub fn create_health_router() -> Router {
    Router::new().route("/health", axum::routing::get(health_check))
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mcp-http-core",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
