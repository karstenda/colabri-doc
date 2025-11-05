# colabri-doc

A Rust server with WebSocket support, REST API, and automatically generated Swagger documentation.

## Features

- **WebSocket Server**: Real-time bidirectional communication at `/ws`
- **REST API**: HTTP endpoints under `/api` route
- **Swagger Documentation**: Auto-generated OpenAPI documentation at `/swagger-ui`

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- Cargo

### Installation

```bash
cargo build
```

### Running the Server

```bash
cargo run
```

The server will start on `http://localhost:3000`

## API Endpoints

### Root
- `GET /` - Landing page with links to all features

### REST API
- `GET /api/health` - Health check endpoint
- `POST /api/items` - Create a new item

### WebSocket
- `WS /ws` - WebSocket endpoint for real-time communication

### Documentation
- `GET /swagger-ui` - Interactive Swagger UI documentation
- `GET /api-docs/openapi.json` - OpenAPI specification

## Usage Examples

### Testing the Health Check API

```bash
curl http://localhost:3000/api/health
```

Response:
```json
{
  "status": "ok",
  "message": "Server is running"
}
```

### Creating an Item

```bash
curl -X POST http://localhost:3000/api/items \
  -H "Content-Type: application/json" \
  -d '{"name": "My Item", "description": "Item description"}'
```

Response:
```json
{
  "id": 1,
  "name": "My Item",
  "description": "Item description"
}
```

### WebSocket Connection

You can test the WebSocket connection using any WebSocket client. The server will:
1. Send a welcome message when you connect
2. Echo back any text messages you send

Example using Python:
```python
import asyncio
import websockets

async def test_websocket():
    uri = "ws://localhost:3000/ws"
    async with websockets.connect(uri) as websocket:
        # Receive welcome message
        welcome = await websocket.recv()
        print(f"Received: {welcome}")
        
        # Send a message
        await websocket.send("Hello, WebSocket!")
        
        # Receive echo
        response = await websocket.recv()
        print(f"Received: {response}")

asyncio.run(test_websocket())
```

## Project Structure

```
colabri-doc/
├── Cargo.toml          # Project dependencies and configuration
├── src/
│   └── main.rs         # Main server implementation
└── README.md           # This file
```

## Dependencies

- **axum**: Web framework with WebSocket support
- **tokio**: Async runtime
- **tower** & **tower-http**: Middleware support
- **serde** & **serde_json**: JSON serialization
- **utoipa**: OpenAPI documentation generation
- **utoipa-swagger-ui**: Swagger UI integration

## Screenshots

### Landing Page
![Landing Page](https://github.com/user-attachments/assets/08e6e39d-4604-420f-a2c4-e803f522cc95)

### Swagger UI
![Swagger UI](https://github.com/user-attachments/assets/215f6233-5227-41e5-92ea-772a3eb02992)

### API Endpoint Details
![API Details](https://github.com/user-attachments/assets/29788de0-76da-4bb2-93b4-5fc0d71bd76b)

## License

MIT