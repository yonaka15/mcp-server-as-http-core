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
    ) -> McpCoreResult<Self> {
        tracing::info!("Initializing MCP HTTP server...");
        tracing::info!(
            "Config file: '{}', Server: '{}'",
            config_file_path,
            server_name
        );

        // Load configuration
        let servers_config = McpServersConfig::load_from_file(config_file_path).await?;
        let server_config = servers_config.get_server(server_name)?.clone();

        // Start MCP server process directly
        let mcp_process = Self::start_mcp_process(&server_config, server_name).await?;

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

    /// Start MCP server process with optional repository clone and build command execution
    async fn start_mcp_process(
        config: &crate::config::McpServerConfig,
        server_name: &str,
    ) -> McpCoreResult<McpProcess> {
        tracing::info!(
            "Starting MCP server '{}': {} {:?}",
            server_name,
            config.command,
            config.args
        );

        // Get server-specific working directory
        let work_dir = Self::get_server_work_dir(server_name);
        tokio::fs::create_dir_all(&work_dir).await.map_err(|e| McpCoreError::ProcessError {
            message: format!("Failed to create work directory '{}': {}", work_dir, e),
        })?;

        // Clone repository if specified and not already exists
        if let Some(repository_url) = &config.repository {
            Self::clone_repository_if_needed(repository_url, &work_dir).await?;
        }

        // Execute build command if present
        if let Some(build_cmd) = &config.build_command {
            tracing::info!("Executing build command: {}", build_cmd);
            Self::execute_build_command(build_cmd, &work_dir, &config.env).await?;
        }

        let mut command_builder = tokio::process::Command::new(&config.command);
        command_builder.args(&config.args);
        command_builder.envs(&config.env);

        // Inherit parent environment variables
        for (key, value) in std::env::vars() {
            command_builder.env(key, value);
        }

        command_builder.current_dir(&work_dir);
        
        command_builder
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        McpProcess::spawn(command_builder).await
    }

    /// Get server-specific working directory path
    fn get_server_work_dir(server_name: &str) -> String {
        format!("/tmp/mcp-servers/{}", server_name)
    }

    /// Clone repository if it doesn't already exist
    async fn clone_repository_if_needed(
        repository_url: &str,
        work_dir: &str,
    ) -> McpCoreResult<()> {
        tracing::info!("Checking repository: {}", repository_url);

        // Check if directory already contains a git repository
        let git_dir = format!("{}/.git", work_dir);
        if tokio::fs::metadata(&git_dir).await.is_ok() {
            tracing::info!("Repository already exists in '{}', skipping clone", work_dir);
            return Ok(());
        }

        tracing::info!("Cloning repository '{}' to '{}'", repository_url, work_dir);

        let start_time = std::time::Instant::now();
        
        // Use git clone command
        let mut command_builder = tokio::process::Command::new("git");
        command_builder.args(["clone", repository_url, "."]);
        command_builder.current_dir(work_dir);
        
        // Capture output for logging
        command_builder
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        tracing::debug!("Executing: git clone {} .", repository_url);

        let output = command_builder
            .output()
            .await
            .map_err(|e| McpCoreError::ProcessError {
                message: format!("Failed to execute git clone: {}", e),
            })?;

        let duration = start_time.elapsed();

        // Log the output
        if !output.stdout.is_empty() {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            tracing::debug!("Git clone stdout: {}", stdout_str.trim());
        }

        if !output.stderr.is_empty() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            if output.status.success() {
                tracing::debug!("Git clone stderr: {}", stderr_str.trim());
            } else {
                tracing::error!("Git clone stderr: {}", stderr_str.trim());
            }
        }

        // Check if the command was successful
        if output.status.success() {
            tracing::info!(
                "Repository cloned successfully in {:?}: {}",
                duration,
                repository_url
            );
            Ok(())
        } else {
            let error_msg = format!(
                "Git clone failed with exit code {:?}: {}",
                output.status.code(),
                repository_url
            );
            tracing::error!("{}", error_msg);
            Err(McpCoreError::ProcessError {
                message: error_msg,
            })
        }
    }

    /// Execute build command in the specified working directory
    async fn execute_build_command(
        build_cmd: &str,
        work_dir: &str,
        env_vars: &std::collections::HashMap<String, String>,
    ) -> McpCoreResult<()> {
        tracing::info!("Starting build process: {}", build_cmd);
        
        // Parse the build command (handle shell commands with &&, ||, etc.)
        let mut command_builder = if cfg!(target_os = "windows") {
            let mut cmd = tokio::process::Command::new("cmd");
            cmd.args(["/C", build_cmd]);
            cmd
        } else {
            let mut cmd = tokio::process::Command::new("sh");
            cmd.args(["-c", build_cmd]);
            cmd
        };

        // Set environment variables
        command_builder.envs(env_vars);
        
        // Inherit parent environment variables
        for (key, value) in std::env::vars() {
            command_builder.env(key, value);
        }
        
        // Set working directory
        command_builder.current_dir(work_dir);
        
        // Capture output for logging
        command_builder
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        tracing::debug!("Executing build command in directory: {}", work_dir);
        
        let start_time = std::time::Instant::now();
        let output = command_builder
            .output()
            .await
            .map_err(|e| McpCoreError::ProcessError {
                message: format!("Failed to execute build command '{}': {}", build_cmd, e),
            })?;

        let duration = start_time.elapsed();
        
        // Log the output
        if !output.stdout.is_empty() {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            tracing::info!("Build stdout: {}", stdout_str.trim());
        }
        
        if !output.stderr.is_empty() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            if output.status.success() {
                tracing::info!("Build stderr: {}", stderr_str.trim());
            } else {
                tracing::error!("Build stderr: {}", stderr_str.trim());
            }
        }
        
        // Check if the command was successful
        if output.status.success() {
            tracing::info!(
                "Build command completed successfully in {:?}: {}",
                duration,
                build_cmd
            );
            Ok(())
        } else {
            let error_msg = format!(
                "Build command failed with exit code {:?}: {}",
                output.status.code(),
                build_cmd
            );
            tracing::error!("{}", error_msg);
            Err(McpCoreError::ProcessError {
                message: error_msg,
            })
        }
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
