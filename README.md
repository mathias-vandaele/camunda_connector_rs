# Camunda Connector Rust

A Rust procedural macro library for easily creating HTTP endpoints that integrate with Camunda BPM. This library allows you to define connector functions with simple attributes and automatically generates the necessary HTTP server infrastructure.

## Features

- 🚀 **Simple API**: Define connector functions with just a few attributes
- 🔄 **Auto-generated endpoints**: Automatically creates REST endpoints for your connectors
- 📦 **Type-safe**: Leverages Rust's type system for request/response handling
- ⚡ **Async support**: Built on top of `tokio` and `axum` for high performance
- 🛠️ **Easy integration**: Minimal boilerplate code required

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
camunda_connector_rs = "0.2.0"
axum = "0.8.4"
inventory = "0.3.20"
serde = { version = "1.0.219", features = ["derive"] }
tokio = {  version = "1.47.1" , features = ["full"]}
```

## Quick Start

Here's a complete example showing how to create math operation connectors:

```rust
use camunda_connector_rs::{camunda_connector, connector_main};
use serde::{Deserialize, Serialize};

// Start the server on port 8080
connector_main!(port = 8080);

#[derive(Deserialize, Debug)]
pub struct MyInput {
    pub a: u64,
    pub b: u64,
}

#[derive(Serialize, Debug)]
pub struct MyOutput {
    pub result: u64,
}

#[camunda_connector(name = "math", operation = "add")]
pub async fn add(id: u64, params: MyInput) -> Result<MyOutput, String> {
    println!("[add] id: {}, params: {:?}", id, params);
    Ok(MyOutput {
        result: params.a + params.b
    })
}

#[camunda_connector(name = "math", operation = "sub")]
pub async fn sub(id: u64, params: MyInput) -> Result<MyOutput, String> {
    println!("[sub] id: {}, params: {:?}", id, params);
    Ok(MyOutput {
        result: params.a - params.b
    })
}
```

## API Reference

### `#[camunda_connector]` Attribute

The main attribute macro for defining connector endpoints.

**Parameters:**
- `name`: The connector name (used in the URL path)
- `operation`: The operation name (used in the URL path)

**Function Requirements:**
- Must be `async`
- Must have exactly 2 parameters:
    1. `id: u64` - The connector execution ID
    2. `params: T` - Your custom input type (must implement `Deserialize`)
- Must return `Result<O, E>` where:
    - `O` implements `Serialize` (success response)
    - `E` implements `Display` (error response)

**Generated Endpoint:**
Each connector generates an endpoint at `/csp/{name}/{operation}` that accepts POST requests.

### `connector_main!` Macro

Generates the main function and HTTP server setup.

**Parameters:**
- `port`: The port number to run the server on

## How It Works

1. **Connector Registration**: Each `#[camunda_connector]` function is automatically registered using the `inventory` crate
2. **Router Generation**: The macro generates an Axum router for each connector endpoint
3. **Request Handling**: Incoming JSON requests are automatically deserialized to your input type
4. **Response Handling**: Function results are serialized to JSON responses
5. **Error Handling**: Errors are automatically converted to HTTP 500 responses

## Generated Code Structure

For each connector function, the macro generates:

- A request struct named `ConnectorRequest{Name}{Operation}`
- An HTTP handler function
- A router builder function
- Registration with the global connector inventory

## Example Endpoints

Based on the example above, the following endpoints would be created:

- `POST /csp/math/add` - Addition operation
- `POST /csp/math/sub` - Subtraction operation

### Request Format

```json
{
  "id": 12345,
  "params": {
    "a": 10,
    "b": 5
  }
}
```

### Response Format

**Success (200 OK):**
```json
{
  "result": 15
}
```

**Error (500 Internal Server Error):**
```
Error message string
```

## Error Handling

The library automatically handles:
- JSON deserialization errors
- Function execution errors (converted to HTTP 500)
- Type validation errors

Your connector functions should return `Result<T, E>` where `E` implements `Display` for proper error messages.

## Advanced Usage

### Custom Input/Output Types

You can use any types that implement the required traits:

```rust
#[derive(Deserialize)]
pub struct DatabaseQuery {
    pub table: String,
    pub conditions: Vec<String>,
}

#[derive(Serialize)]
pub struct DatabaseResult {
    pub rows: Vec<serde_json::Value>,
    pub count: usize,
}

#[camunda_connector(name = "database", operation = "query")]
pub async fn query_database(id: u64, params: DatabaseQuery) -> Result<DatabaseResult, String> {
    // Your database logic here
    todo!()
}
```

### Multiple Connectors

You can define as many connectors as needed in the same application:

```rust
#[camunda_connector(name = "email", operation = "send")]
pub async fn send_email(id: u64, params: EmailParams) -> Result<EmailResult, String> {
    // Email sending logic
}

#[camunda_connector(name = "slack", operation = "notify")]
pub async fn slack_notify(id: u64, params: SlackParams) -> Result<SlackResult, String> {
    // Slack notification logic
}
```

## Dependencies

This library uses the following key dependencies:

- [`syn`](https://crates.io/crates/syn) & [`quote`](https://crates.io/crates/quote) - Proc macro utilities

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Changelog

### v0.1.0
- Initial release
- Basic connector macro functionality
- HTTP server generation
- Error handling support