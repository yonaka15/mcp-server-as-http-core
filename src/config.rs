//! Configuration management for MCP HTTP Core

use crate::error::{McpCoreError, McpCoreResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration structure for MCP servers
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpServersConfig {
    /// Version of the configuration format
    #[serde(default = "default_version")]
    pub version: String,

    /// Map of server name to server configuration
    pub servers: HashMap<String, McpServerConfig>,
}

/// Configuration for a single MCP server
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpServerConfig {
    /// Git repository URL (optional)
    pub repository: Option<String>,

    /// Build command to execute after cloning (optional)
    pub build_command: Option<String>,

    /// Command to execute the MCP server
    pub command: String,

    /// Arguments for the command
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables for the process
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Runtime-specific configuration
    #[serde(default)]
    pub runtime_config: RuntimeConfig,
}

/// Runtime-specific configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RuntimeConfig {
    /// Node.js specific configuration
    pub node: Option<NodeConfig>,

    /// Python specific configuration  
    pub python: Option<PythonConfig>,

    /// Go specific configuration
    pub go: Option<GoConfig>,
}

/// Node.js runtime configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeConfig {
    /// Node.js version requirement
    pub version: Option<String>,

    /// Package manager (npm, yarn, pnpm)
    pub package_manager: Option<String>,

    /// Additional npm/yarn flags
    pub install_flags: Option<Vec<String>>,
}

/// Python runtime configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PythonConfig {
    /// Python version requirement
    pub version: Option<String>,

    /// Virtual environment path
    pub venv_path: Option<String>,

    /// Requirements file path
    pub requirements_file: Option<String>,
}

/// Go runtime configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoConfig {
    /// Go version requirement
    pub version: Option<String>,

    /// Go module path
    pub module_path: Option<String>,

    /// Build flags
    pub build_flags: Option<Vec<String>>,
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// API key for Bearer token authentication
    pub api_key: Option<String>,

    /// Whether authentication is enabled
    pub enabled: bool,
}

impl Default for McpServersConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            servers: HashMap::new(),
        }
    }
}

impl AuthConfig {
    /// Create AuthConfig from environment variables
    pub fn from_env() -> Self {
        let api_key = std::env::var("HTTP_API_KEY").ok();
        let disable_auth = std::env::var("DISABLE_AUTH")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let enabled = !disable_auth && api_key.is_some();

        Self { api_key, enabled }
    }
}

impl McpServersConfig {
    /// Load configuration from file
    pub async fn load_from_file(path: &str) -> McpCoreResult<Self> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            McpCoreError::ConfigurationError {
                message: format!("Failed to read config file '{}': {}", path, e),
            }
        })?;

        let config: McpServersConfig =
            serde_json::from_str(&content).map_err(|e| McpCoreError::ConfigurationError {
                message: format!("Failed to parse config file '{}': {}", path, e),
            })?;

        Ok(config)
    }

    /// Get server configuration by name
    pub fn get_server(&self, name: &str) -> McpCoreResult<&McpServerConfig> {
        self.servers
            .get(name)
            .ok_or_else(|| McpCoreError::ConfigurationError {
                message: format!("Server configuration not found for '{}'", name),
            })
    }
}

fn default_version() -> String {
    "1.0".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_from_env() {
        std::env::set_var("HTTP_API_KEY", "test-key");
        std::env::set_var("DISABLE_AUTH", "false");

        let config = AuthConfig::from_env();
        assert!(config.enabled);
        assert_eq!(config.api_key, Some("test-key".to_string()));

        std::env::remove_var("HTTP_API_KEY");
        std::env::remove_var("DISABLE_AUTH");
    }
}
