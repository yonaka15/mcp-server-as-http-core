# MCP Server as HTTP Core

A high-performance Rust-based HTTP server that provides a REST API interface to Model Context Protocol (MCP) servers. This is the core library that can be extended by language-specific implementations.

## Features

- **High Performance**: Built with Rust and Axum for optimal performance
- **Runtime Abstraction**: Support for Node.js, Python, and Go MCP servers
- **Authentication**: Bearer token authentication with configurable security
- **Process Management**: Robust MCP server process communication
- **Configuration**: Flexible JSON-based configuration system
- **Repository Management**: Automatic Git repository cloning and building
- **Observability**: Comprehensive logging with tracing

## Architecture

This core library provides:

- **HTTP Server**: Axum-based REST API server
- **Authentication**: Bearer token validation middleware
- **Runtime Interface**: Abstraction for different language runtimes
- **Process Communication**: Async stdin/stdout communication with MCP servers
- **Configuration Management**: JSON schema for server configuration

## Quick Start

### Basic Usage

```bash
# Set environment variables
export MCP_CONFIG_FILE=mcp_servers.config.json
export MCP_SERVER_NAME=your-server
export MCP_RUNTIME_TYPE=node
export PORT=3000

# Run the server
cargo run
```

### Configuration

Create `mcp_servers.config.json`:

```json
{
  "version": "1.0",
  "servers": {
    "example-server": {
      "repository": "https://github.com/user/mcp-server",
      "build_command": "npm install && npm run build",
      "command": "node",
      "args": ["dist/index.js"],
      "env": {
        "CUSTOM_VAR": "value"
      },
      "runtime_config": {
        "node": {
          "version": ">=18.0.0",
          "package_manager": "npm"
        }
      }
    }
  }
}
```

### Environment Variables

- `HTTP_API_KEY`: Bearer token for authentication (optional)
- `DISABLE_AUTH`: Set to "true" to disable authentication
- `MCP_CONFIG_FILE`: Path to configuration file (default: "mcp_servers.config.json")
- `MCP_SERVER_NAME`: Server name from config to use
- `MCP_RUNTIME_TYPE`: Runtime type (node, python, go)
- `PORT`: HTTP server port (default: 3000)
- `RUST_LOG`: Log level configuration

## API Usage

### Authentication

Include Bearer token in Authorization header:

```bash
curl -X POST http://localhost:3000/api/v1 \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{"command": "{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"tools/list\", \"params\": {}}"}'
```

### Example Request

```bash
curl -X POST http://localhost:3000/api/v1 \
  -H "Content-Type: application/json" \
  -d '{"command": "{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"tools/list\", \"params\": {}}"}'
```

## Development

### Building

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run
```

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## Runtime Support

### Node.js Runtime
- Automatic npm/yarn dependency installation
- TypeScript compilation support
- Environment variable passthrough

### Python Runtime (Planned)
- Virtual environment management
- pip/poetry dependency installation
- Python 3.8+ support

### Go Runtime (Planned)
- Go module support
- Automatic dependency resolution
- Cross-compilation support

## Language-Specific Repositories

This core library is designed to be used by language-specific repositories:

- `mcp-server-as-http-node`: Node.js/TypeScript optimized implementation
- `mcp-server-as-http-python`: Python optimized implementation  
- `mcp-server-as-http-go`: Go optimized implementation

Each language-specific repository provides:
- Optimized Docker images
- Language-specific dependency management
- Runtime-specific optimizations
- Specialized configuration options

## Error Handling

The library provides comprehensive error handling with detailed error messages:

- `AuthenticationError`: Authentication failures
- `ConfigurationError`: Configuration parsing issues
- `ProcessError`: MCP server communication problems
- `RuntimeError`: Runtime setup failures
- `HttpServerError`: HTTP server issues

## Logging

Structured logging with tracing:

```bash
# Set log level
export RUST_LOG=debug

# Filter by module
export RUST_LOG=mcp_server_as_http_core=debug

# Multiple filters
export RUST_LOG=mcp_server_as_http_core=debug,axum=info
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Write tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.
