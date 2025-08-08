# Camunda Connector Rust

A Rust procedural macro library for building Camunda connectors with automatic request dispatching and type-safe handlers.

## Features

- **Declarative Connector Definition**: Use attributes to define connectors with minimal boilerplate
- **Automatic Request Dispatching**: Built-in HTTP server with routing based on connector name and operation
- **Type Safety**: Fully typed request/response handling with automatic serialization/deserialization
- **Async Support**: Built on Tokio and Axum for high-performance async operations
- **Inventory Integration**: Automatic registration of connectors at compile time

## Quick Start

Add the following dependencies to your `Cargo.toml`:

```toml
[dependencies]
camunda_connector_rs = "0.0.3"
axum = "0.8.4"
inventory = "0.3.20"
serde = { version = "1.0.219", features = ["derive"] }
tokio = {  version = "1.47.1" , features = ["full"]}
serde_json = "1.0.142"
```

### Basic Example

```rust
use camunda_connector_rs::{camunda_connector, connector_main};
use serde::{Deserialize, Serialize};

// Start the server on port 8080
connector_main!(port = 8080);

// Define your input/output types
#[derive(Deserialize, Debug)]
pub struct MathInput {
    pub a: u64,
    pub b: u64,
}

#[derive(Serialize, Debug)]
pub struct MathOutput {
    pub result: u64,
}

// Define connector handlers
#[camunda_connector(name = "math", operation = "add")]
pub async fn add(id: u64, params: MathInput) -> Result<MathOutput, String> {
    println!("[add] id: {}, params: {:?}", id, params);
    Ok(MathOutput {
        result: params.a + params.b,
    })
}

#[camunda_connector(name = "math", operation = "subtract")]
pub async fn subtract(id: u64, params: MathInput) -> Result<MathOutput, String> {
    println!("[subtract] id: {}, params: {:?}", id, params);
    Ok(MathOutput {
        result: params.a - params.b,
    })
}
```

## Usage

### Defining Connectors

Use the `#[camunda_connector]` attribute to define connector handlers:

```rust
#[camunda_connector(name = "connector_name", operation = "operation_name")]
pub async fn handler_function(id: u64, params: InputType) -> Result<OutputType, String> {
    // Your connector logic here
}
```

**Requirements:**
- Functions must be `async`
- Must take exactly 2 parameters: `id: u64` and `params: T` where `T` implements `Deserialize`
- Must return `Result<T, String>` where `T` implements `Serialize`

### Starting the Server

Use the `connector_main!` macro to generate the main function and start the HTTP server:

```rust
connector_main!(port = 8080);
```

This generates:
- A complete `main()` function with Tokio runtime
- HTTP server using Axum
- Request routing and dispatching logic
- Error handling and JSON response formatting

### Request Format

Connectors expect HTTP POST requests to `/csp/{connector_name}` with JSON payload:

```json
{
    "id": 12345,
    "params": {
        "operation": "operation_name",
        "input": {
            // Your connector-specific input data
        }
    }
}
```

### Response Format

Successful responses return JSON with your connector's output data. Error responses return appropriate HTTP status codes with error messages.

## How It Works

### Code Generation

The `#[camunda_connector]` attribute generates:

1. **Request/Response Structs**: Automatically creates typed structs for deserialization
   ```rust
   // Generated for each connector
   pub struct ParamsMathAdd { /* ... */ }
   pub struct RequestMathAdd { /* ... */ }
   ```

2. **Dispatcher Function**: Creates a type-safe dispatcher that handles JSON parsing and calls your handler
   ```rust
   fn exec_raw_math_add(bytes: axum::body::Bytes) -> DispatcherFuture { /* ... */ }
   ```

3. **Registration**: Uses the `inventory` crate to register connectors at compile time

### Request Flow

1. HTTP request arrives at `/csp/{connector_name}`
2. Server peeks at the `operation` field to determine routing
3. Finds the appropriate handler function using a lookup table
4. Deserializes the full request with proper typing
5. Calls your handler function
6. Serializes and returns the response

## Error Handling

The framework provides comprehensive error handling:

- **JSON Parsing Errors**: Returns 400 Bad Request for malformed JSON
- **Unknown Connectors**: Returns 400 Bad Request for unregistered connector/operation combinations
- **Handler Errors**: Returns 500 Internal Server Error for errors returned by your handler functions

## Advanced Features

### Multiple Operations per Connector

You can define multiple operations for the same connector:

```rust
#[camunda_connector(name = "calculator", operation = "add")]
pub async fn add(id: u64, params: MathInput) -> Result<MathOutput, String> { /* ... */ }

#[camunda_connector(name = "calculator", operation = "multiply")]
pub async fn multiply(id: u64, params: MathInput) -> Result<MathOutput, String> { /* ... */ }
```

### Custom Input/Output Types

Each connector operation can have its own input and output types:

```rust
#[derive(Deserialize)]
pub struct StringInput {
    pub text: String,
}

#[derive(Serialize)]
pub struct StringOutput {
    pub length: usize,
    pub uppercase: String,
}

#[camunda_connector(name = "string_utils", operation = "process")]
pub async fn process_string(id: u64, params: StringInput) -> Result<StringOutput, String> {
    Ok(StringOutput {
        length: params.text.len(),
        uppercase: params.text.to_uppercase(),
    })
}
```

## Dependencies

This library uses the following key dependencies:

- **proc-macro2, quote, syn**: For procedural macro implementation
- **serde**: For JSON serialization/deserialization
- **axum**: For HTTP server functionality
- **tokio**: For async runtime
- **inventory**: For compile-time connector registration

## License

[Add your license information here]

## Contributing

[Add contribution guidelines here]