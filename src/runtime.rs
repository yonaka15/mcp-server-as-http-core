//! Runtime abstraction for different MCP server types

use crate::config::{McpServerConfig, RuntimeConfig};
use crate::error::{McpCoreError, McpCoreResult};
use crate::process::McpProcess;
use async_trait::async_trait;

/// Runtime interface for managing MCP servers in different languages
#[async_trait]
pub trait McpRuntime: Send + Sync {
    /// Setup the runtime environment (install dependencies, etc.)
    async fn setup_environment(&self, config: &RuntimeConfig) -> McpCoreResult<()>;

    /// Clone and build a repository if specified
    async fn setup_repository(
        &self,
        config: &McpServerConfig,
        work_dir: &str,
    ) -> McpCoreResult<String>;

    /// Start the MCP server process
    async fn start_server(
        &self,
        config: &McpServerConfig,
        working_dir: &str,
    ) -> McpCoreResult<McpProcess>;
}

/// Node.js runtime implementation
pub struct NodeRuntime;

/// Python runtime implementation  
pub struct PythonRuntime;

/// Go runtime implementation
pub struct GoRuntime;

#[async_trait]
impl McpRuntime for NodeRuntime {
    async fn setup_environment(&self, _config: &RuntimeConfig) -> McpCoreResult<()> {
        // Node.js environment setup
        tracing::info!("Setting up Node.js environment");

        // Check if Node.js is available
        let output = tokio::process::Command::new("node")
            .arg("--version")
            .output()
            .await
            .map_err(|e| McpCoreError::RuntimeError {
                message: format!("Node.js not found: {}", e),
            })?;

        if !output.status.success() {
            return Err(McpCoreError::RuntimeError {
                message: "Node.js is not available".to_string(),
            });
        }

        let version = String::from_utf8_lossy(&output.stdout);
        tracing::info!("Node.js version: {}", version.trim());

        Ok(())
    }

    async fn setup_repository(
        &self,
        config: &McpServerConfig,
        work_dir: &str,
    ) -> McpCoreResult<String> {
        if let Some(repo_url) = &config.repository {
            tracing::info!("Cloning Node.js repository: {}", repo_url);

            // Extract repository name
            let repo_name =
                repo_url
                    .split('/')
                    .last()
                    .ok_or_else(|| McpCoreError::RuntimeError {
                        message: "Invalid repository URL".to_string(),
                    })?;

            let clone_path = format!("{}/{}", work_dir, repo_name);

            // Remove existing directory if it exists
            if tokio::fs::metadata(&clone_path).await.is_ok() {
                tracing::debug!("Removing existing directory: {}", clone_path);
                tokio::fs::remove_dir_all(&clone_path).await.map_err(|e| {
                    McpCoreError::RuntimeError {
                        message: format!("Failed to remove existing directory: {}", e),
                    }
                })?;
            }

            // Execute git clone
            let clone_output = tokio::process::Command::new("git")
                .args(["clone", repo_url, &clone_path])
                .output()
                .await
                .map_err(|e| McpCoreError::RuntimeError {
                    message: format!("Failed to execute git clone: {}", e),
                })?;

            if !clone_output.status.success() {
                let error_msg = String::from_utf8_lossy(&clone_output.stderr);
                return Err(McpCoreError::RuntimeError {
                    message: format!("Git clone failed: {}", error_msg),
                });
            }

            tracing::info!("Repository cloned to: {}", clone_path);

            // Execute build command if specified
            if let Some(build_cmd) = &config.build_command {
                tracing::info!("Executing build command: {}", build_cmd);

                let mut build_command = tokio::process::Command::new("sh");
                build_command.args(["-c", build_cmd]);
                build_command.current_dir(&clone_path);

                // Add environment variables from config file
                build_command.envs(&config.env);

                // Inherit parent environment variables
                for (key, value) in std::env::vars() {
                    build_command.env(key, value);
                }

                let build_output =
                    build_command
                        .output()
                        .await
                        .map_err(|e| McpCoreError::RuntimeError {
                            message: format!("Failed to execute build command: {}", e),
                        })?;

                if !build_output.status.success() {
                    let error_msg = String::from_utf8_lossy(&build_output.stderr);
                    return Err(McpCoreError::RuntimeError {
                        message: format!("Build failed: {}", error_msg),
                    });
                }

                tracing::info!("Build completed successfully");
            }

            Ok(clone_path)
        } else {
            // No repository specified, use current directory
            Ok(work_dir.to_string())
        }
    }

    async fn start_server(
        &self,
        config: &McpServerConfig,
        working_dir: &str,
    ) -> McpCoreResult<McpProcess> {
        tracing::info!(
            "Starting Node.js MCP server: {} {:?}",
            config.command,
            config.args
        );

        let mut command_builder = tokio::process::Command::new(&config.command);
        command_builder.args(&config.args);

        // Add environment variables from config file
        command_builder.envs(&config.env);

        // Inherit parent environment variables
        for (key, value) in std::env::vars() {
            command_builder.env(key, value);
        }

        command_builder.current_dir(working_dir);
        command_builder
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        McpProcess::spawn(command_builder).await
    }
}

#[async_trait]
impl McpRuntime for PythonRuntime {
    async fn setup_environment(&self, _config: &RuntimeConfig) -> McpCoreResult<()> {
        tracing::info!("Setting up Python environment");

        // Check if Python is available
        let output = tokio::process::Command::new("python3")
            .arg("--version")
            .output()
            .await
            .map_err(|e| McpCoreError::RuntimeError {
                message: format!("Python3 not found: {}", e),
            })?;

        if !output.status.success() {
            return Err(McpCoreError::RuntimeError {
                message: "Python3 is not available".to_string(),
            });
        }

        let version = String::from_utf8_lossy(&output.stdout);
        tracing::info!("Python version: {}", version.trim());

        Ok(())
    }

    async fn setup_repository(
        &self,
        _config: &McpServerConfig,
        work_dir: &str,
    ) -> McpCoreResult<String> {
        // Similar to Node.js implementation but with Python-specific build commands
        // TODO: Implement Python-specific repository setup
        tracing::warn!("Python repository setup not yet implemented");
        Ok(work_dir.to_string())
    }

    async fn start_server(
        &self,
        config: &McpServerConfig,
        working_dir: &str,
    ) -> McpCoreResult<McpProcess> {
        tracing::info!(
            "Starting Python MCP server: {} {:?}",
            config.command,
            config.args
        );

        let mut command_builder = tokio::process::Command::new(&config.command);
        command_builder.args(&config.args);
        command_builder.envs(&config.env);

        for (key, value) in std::env::vars() {
            command_builder.env(key, value);
        }

        command_builder.current_dir(working_dir);
        command_builder
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        McpProcess::spawn(command_builder).await
    }
}

#[async_trait]
impl McpRuntime for GoRuntime {
    async fn setup_environment(&self, _config: &RuntimeConfig) -> McpCoreResult<()> {
        tracing::info!("Setting up Go environment");

        // Check if Go is available
        let output = tokio::process::Command::new("go")
            .arg("version")
            .output()
            .await
            .map_err(|e| McpCoreError::RuntimeError {
                message: format!("Go not found: {}", e),
            })?;

        if !output.status.success() {
            return Err(McpCoreError::RuntimeError {
                message: "Go is not available".to_string(),
            });
        }

        let version = String::from_utf8_lossy(&output.stdout);
        tracing::info!("Go version: {}", version.trim());

        Ok(())
    }

    async fn setup_repository(
        &self,
        _config: &McpServerConfig,
        work_dir: &str,
    ) -> McpCoreResult<String> {
        // TODO: Implement Go-specific repository setup
        tracing::warn!("Go repository setup not yet implemented");
        Ok(work_dir.to_string())
    }

    async fn start_server(
        &self,
        config: &McpServerConfig,
        working_dir: &str,
    ) -> McpCoreResult<McpProcess> {
        tracing::info!(
            "Starting Go MCP server: {} {:?}",
            config.command,
            config.args
        );

        let mut command_builder = tokio::process::Command::new(&config.command);
        command_builder.args(&config.args);
        command_builder.envs(&config.env);

        for (key, value) in std::env::vars() {
            command_builder.env(key, value);
        }

        command_builder.current_dir(working_dir);
        command_builder
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        McpProcess::spawn(command_builder).await
    }
}

/// Runtime factory for creating appropriate runtime instances
pub fn create_runtime(runtime_type: &str) -> McpCoreResult<Box<dyn McpRuntime>> {
    match runtime_type.to_lowercase().as_str() {
        "node" | "nodejs" | "javascript" | "typescript" => Ok(Box::new(NodeRuntime)),
        "python" | "python3" | "py" => Ok(Box::new(PythonRuntime)),
        "go" | "golang" => Ok(Box::new(GoRuntime)),
        _ => Err(McpCoreError::RuntimeError {
            message: format!("Unsupported runtime type: {}", runtime_type),
        }),
    }
}
