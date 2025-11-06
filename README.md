# colabri-doc

A Rust server with WebSocket support, REST API, and automatically generated Swagger documentation.

## Features

- **WebSocket Server**: Real-time bidirectional communication at `/ws`
- **REST API**: HTTP endpoints under `/api` route
- **Swagger Documentation**: Auto-generated OpenAPI documentation at `/swagger`

## Getting Started

### Prerequisites

- Rust 1.82 or higher
- Cargo

### Installation

```bash
cargo build
```

### Configuration

The application can be configured using environment variables or an `app.env` file. If an `app.env` file exists, it will be loaded automatically. Otherwise, the application will look for a standard `.env` file or use environment variables directly.

#### Configuration Options

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `HOST` | Server host address | `0.0.0.0` | `127.0.0.1` |
| `PORT` | Server port | `3000` | `8080` |
| `ENVIRONMENT` | Environment mode | `development` | `production`, `staging` |
| `LOG_LEVEL` | Logging level | `info` | `debug`, `warn`, `error` |
| `CORS_ORIGINS` | Allowed CORS origins (optional) | None | `http://localhost:3000,https://example.com` |
| `CLOUD_SERVICE_NAME` | Name of the service | `colabri-doc` | `colabri-doc` |
| `CLOUD_POD` | Identifier of the pod | None | `windy-winipeg` |
| `CLOUD_AUTH_JWT_SECRET` | JWT secret | None | `your-secret-key` |
| `GCP_PROJECT_ID` | Google Cloud Project ID | None | `google-cloud-project-id` |
| `DB_URL` | Database connection string | None | `postgresql://user:pass@localhost/db` |

#### Setting up Configuration

1. **Using app.env file (recommended)**:
   ```bash
   cp app.env.example app.env
   # Edit app.env with your preferred settings
   ```

2. **Using environment variables**:
   ```bash
   export HOST=127.0.0.1
   export PORT=8080
   export LOG_LEVEL=debug
   cargo run
   ```

3. **Using .env file**:
   ```bash
   # Create .env file with your configuration
   echo "PORT=8080" > .env
   echo "HOST=127.0.0.1" >> .env
   ```

#### Configuration Priority

The configuration is loaded in the following order (highest to lowest priority):
1. Environment variables
2. `app.env` file
3. `.env` file
4. Default values

### Running the Server

```bash
cargo run
```

The server will start on the configured host and port (default: `http://0.0.0.0:3000`)

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
├── app.env.example     # Example configuration file
├── app.env             # Configuration file (ignored by git)
├── src/
│   ├── main.rs         # Main server implementation
│   ├── config.rs       # Configuration module
│   ├── models/         # Data models
│   ├── handlers/       # Request handlers
│   ├── routes/         # Route definitions
│   ├── docs/           # API documentation
│   └── websocket/      # WebSocket implementation
└── README.md           # This file
```

## Dependencies

- **axum**: Web framework with WebSocket support
- **tokio**: Async runtime
- **tower** & **tower-http**: Middleware support
- **serde** & **serde_json**: JSON serialization
- **utoipa**: OpenAPI documentation generation
- **utoipa-swagger-ui**: Swagger UI integration
- **dotenvy**: Environment file (.env) loading
- **envy**: Environment variable deserialization

## Screenshots

### Landing Page
![Landing Page](https://github.com/user-attachments/assets/08e6e39d-4604-420f-a2c4-e803f522cc95)

### Swagger UI
![Swagger UI](https://github.com/user-attachments/assets/215f6233-5227-41e5-92ea-772a3eb02992)

### API Endpoint Details
![API Details](https://github.com/user-attachments/assets/29788de0-76da-4bb2-93b4-5fc0d71bd76b)

## License

MIT