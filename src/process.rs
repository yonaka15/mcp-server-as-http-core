// Path: src/process.rs
// Compare this snippet from src/process.rs:
//         // Read response with timeout
//         let response_result = timeout(Duration::from_secs(30), async {
//             let mut response_line = String::new();
//             match self.stdout.read_line(&mut response_line).await {

// This is the MCP server process wrapper
use crate::error::{McpCoreError, McpCoreResult};
use serde::{Deserialize, Serialize};
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

    /// Send a query to the MCP server and wait for response
    pub async fn query(&mut self, request: &McpRequest) -> McpCoreResult<McpResponse> {
        let start_time = Instant::now();
        tracing::debug!("Starting MCP query");
        tracing::debug!("Request: {:?}", request);

        // Serialize the request
        let request_json =
            serde_json::to_string(request).map_err(|e| McpCoreError::ProcessError {
                message: format!("Failed to serialize request: {}", e),
            })?;

        tracing::debug!("Serialized request: {}", request_json);

        // Send the command to MCP server (the command field contains the JSON-RPC message)
        let mcp_message = &request.command;
        tracing::debug!("Sending to MCP server: {}", mcp_message);

        // Write to MCP server stdin
        self.stdin
            .write_all((mcp_message.to_string() + "
").as_bytes())
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

        // Read response with timeout
        let response_result = timeout(Duration::from_secs(3600), async {
            let mut response_line = String::new();
            match self.stdout.read_line(&mut response_line).await {
                Ok(0) => {
                    tracing::debug!("MCP server closed connection (EOF)");
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

                    // Return response as string (don't re-serialize as JSON)
                    Ok(McpResponse {
                        result: response_line.trim().to_string(),
                    })
                }
                Err(e) => {
                    tracing::debug!("Error reading from MCP stdout: {}", e);
                    Err(McpCoreError::ProcessError {
                        message: format!("Failed to read from MCP stdout: {}", e),
                    })
                }
            }
        })
        .await;

        match response_result {
            Ok(result) => {
                let elapsed = start_time.elapsed();
                tracing::debug!("MCP query completed in {:?}", elapsed);
                result
            }
            Err(_) => {
                tracing::debug!("MCP query timed out after 3600 seconds");
                Err(McpCoreError::ProcessError {
                    message: "MCP server response timeout (3600 seconds)".to_string(),
                })
            }
        }
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
