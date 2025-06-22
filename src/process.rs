// This is the MCP server process wrapper
use crate::error::{McpCoreError, McpCoreResult};
use serde::{Deserialize, Serialize};
use serde_json;
use std::time::Instant;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, ChildStdout, Command},
    time::{timeout, Duration},
};

/// MCP server process wrapper
pub struct McpProcess {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// MCP request structure
#[derive(Serialize, Deserialize, Debug)]
pub struct McpRequest {
    pub command: String,
}

/// MCP response structure
#[derive(Serialize, Deserialize, Debug)]
pub struct McpResponse {
    pub result: String,
}

impl McpProcess {
    /// Spawn a new MCP process from a command builder
    pub async fn spawn(mut command_builder: Command) -> McpCoreResult<Self> {
        tracing::debug!("Spawning MCP process...");

        let mut child = command_builder
            .spawn()
            .map_err(|e| McpCoreError::ProcessError {
                message: format!("Failed to spawn MCP process: {}", e),
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpCoreError::ProcessError {
                message: "Failed to open stdin for MCP process".to_string(),
            })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpCoreError::ProcessError {
                message: "Failed to open stdout for MCP process".to_string(),
            })?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| McpCoreError::ProcessError {
                message: "Failed to open stderr for MCP process".to_string(),
            })?;

        // Spawn stderr monitoring task
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        tracing::debug!("MCP server stderr: EOF, task finishing");
                        break;
                    }
                    Ok(_) => {
                        tracing::debug!("MCP server stderr: {}", line.trim());
                        line.clear();
                    }
                    Err(e) => {
                        tracing::error!("MCP server stderr read error: {}", e);
                        break;
                    }
                }
            }
        });

        tracing::debug!("MCP process spawned successfully");

        Ok(Self {
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    /// Initialize MCP connection with handshake according to official specification
    pub async fn initialize(&mut self) -> McpCoreResult<()> {
        tracing::info!("Initializing MCP connection...");
        
        // Send initialize request with proper capabilities structure per MCP specification
        let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "init",
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    // Client capabilities - properly structured
                    "roots": {
                        "listChanged": false
                    },
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "mcp-http-core",
                    "title": "MCP HTTP Core",
                    "version": "0.1.0"
                }
            }
        });
        
        let init_message = init_request.to_string();
        tracing::debug!("Sending initialize request: {}", init_message);
        
        // Send initialize
        self.stdin
            .write_all((init_message + "\n").as_bytes())
            .await
            .map_err(|e| McpCoreError::ProcessError {
                message: format!("Failed to write initialize request: {}", e),
            })?;
            
        self.stdin
            .flush()
            .await
            .map_err(|e| McpCoreError::ProcessError {
                message: format!("Failed to flush initialize request: {}", e),
            })?;
            
        // Wait for initialize response
        let init_response = self.read_response_with_timeout(Duration::from_secs(30)).await?;
        tracing::debug!("Initialize response: {}", init_response);
        
        // Parse and validate the response
        match serde_json::from_str::<serde_json::Value>(&init_response) {
            Ok(response) => {
                if let Some(error) = response.get("error") {
                    return Err(McpCoreError::ProcessError {
                        message: format!("MCP initialization error: {}", error),
                    });
                }
                
                if let Some(result) = response.get("result") {
                    if let Some(protocol_version) = result.get("protocolVersion") {
                        tracing::info!("Server protocol version: {}", protocol_version);
                    }
                    if let Some(capabilities) = result.get("capabilities") {
                        tracing::info!("Server capabilities: {}", capabilities);
                    }
                    if let Some(server_info) = result.get("serverInfo") {
                        tracing::info!("Server info: {}", server_info);
                    }
                } else {
                    tracing::warn!("Initialize response missing 'result' field");
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse initialize response as JSON: {}", e);
                // Continue anyway - some servers might send non-JSON responses
            }
        }
        
        // Send initialized notification per MCP specification
        let initialized_notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        
        let notification_message = initialized_notification.to_string();
        tracing::debug!("Sending initialized notification: {}", notification_message);
        
        self.stdin
            .write_all((notification_message + "\n").as_bytes())
            .await
            .map_err(|e| McpCoreError::ProcessError {
                message: format!("Failed to write initialized notification: {}", e),
            })?;
            
        self.stdin
            .flush()
            .await
            .map_err(|e| McpCoreError::ProcessError {
                message: format!("Failed to flush initialized notification: {}", e),
            })?;
            
        tracing::info!("MCP connection initialized successfully");
        Ok(())
    }
    
    /// Read a single response from MCP server with timeout
    async fn read_response_with_timeout(&mut self, timeout_duration: Duration) -> McpCoreResult<String> {
        let response_result = timeout(timeout_duration, async {
            let mut response_line = String::new();
            match self.stdout.read_line(&mut response_line).await {
                Ok(0) => {
                    tracing::warn!("MCP server closed connection (EOF)");
                    Err(McpCoreError::ProcessError {
                        message: "MCP server closed the connection (EOF)".to_string(),
                    })
                }
                Ok(bytes_read) => {
                    tracing::debug!("Read {} bytes from MCP server", bytes_read);
                    tracing::debug!("Raw response: '{}'", response_line.trim());

                    if response_line.trim().is_empty() {
                        return Err(McpCoreError::ProcessError {
                            message: "MCP server returned an empty line".to_string(),
                        });
                    }

                    Ok(response_line.trim().to_string())
                }
                Err(e) => {
                    tracing::error!("Error reading from MCP stdout: {}", e);
                    Err(McpCoreError::ProcessError {
                        message: format!("Failed to read from MCP stdout: {}", e),
                    })
                }
            }
        })
        .await;

        match response_result {
            Ok(result) => result,
            Err(_) => {
                let timeout_secs = timeout_duration.as_secs();
                tracing::error!("MCP server response timeout after {} seconds", timeout_secs);
                Err(McpCoreError::ProcessError {
                    message: format!("MCP server response timeout ({} seconds)", timeout_secs),
                })
            }
        }
    }

    /// Send a query to the MCP server and wait for response
    pub async fn query(&mut self, request: &McpRequest) -> McpCoreResult<McpResponse> {
        let start_time = Instant::now();
        tracing::debug!("Starting MCP query");
        tracing::debug!("Request: {:?}", request);

        // Send the command to MCP server (the command field contains the JSON-RPC message)
        let mcp_message = &request.command;
        tracing::debug!("Sending to MCP server: {}", mcp_message);

        // Write to MCP server stdin
        self.stdin
            .write_all((mcp_message.to_string() + "\n").as_bytes())
            .await
            .map_err(|e| McpCoreError::ProcessError {
                message: format!("Failed to write to MCP stdin: {}", e),
            })?;

        self.stdin
            .flush()
            .await
            .map_err(|e| McpCoreError::ProcessError {
                message: format!("Failed to flush MCP stdin: {}", e),
            })?;

        tracing::debug!("Data sent to MCP server, waiting for response...");

        // Read response with shorter timeout for regular queries
        let response_line = self.read_response_with_timeout(Duration::from_secs(30)).await?;
        
        let elapsed = start_time.elapsed();
        tracing::debug!("MCP query completed in {:?}", elapsed);
        
        Ok(McpResponse {
            result: response_line,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_request_serialization() {
        let request = McpRequest {
            command: r#"{"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}"#
                .to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("command"));
        assert!(json.contains("tools/list"));
    }

    #[test]
    fn test_mcp_response_serialization() {
        let response = McpResponse {
            result: r#"{"jsonrpc": "2.0", "id": 1, "result": {"tools": []}}"#.to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("result"));
        assert!(json.contains("tools"));
    }
}
