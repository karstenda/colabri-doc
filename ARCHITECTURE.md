# Colabri-Doc API Structure

This document outlines the refactored structure of the Colabri-Doc API project.

## Project Structure

```
src/
├── main.rs           # Application entry point and server configuration
├── models/           # Data structures and schemas
│   ├── mod.rs       # Module exports
│   ├── health.rs    # Health check response models
│   └── item.rs      # Item-related request/response models
├── handlers/         # API endpoint implementations
│   ├── mod.rs       # Module exports
│   ├── health.rs    # Health check handler
│   └── item.rs      # Item-related handlers
├── routes/          # Route definitions
│   ├── mod.rs       # Module exports
│   └── api.rs       # API route configuration
├── websocket/       # WebSocket functionality
│   ├── mod.rs       # Module exports
│   └── handler.rs   # WebSocket connection handling
└── docs/            # API documentation
    └── mod.rs       # OpenAPI documentation generation
```

## Module Responsibilities

### Models (`src/models/`)
Contains all data structures used for API requests and responses:
- **health.rs**: Health check response structure
- **item.rs**: Item creation request and response structures
- All models implement `Serialize`, `Deserialize`, and `ToSchema` for OpenAPI documentation

### Handlers (`src/handlers/`)
Contains the business logic for API endpoints:
- **health.rs**: Health check endpoint implementation
- **item.rs**: Item creation endpoint implementation
- Handlers are pure functions that take requests and return responses

### Routes (`src/routes/`)
Contains route configuration and URL mapping:
- **api.rs**: Defines API routes and maps them to handlers
- Keeps routing logic separate from business logic

### WebSocket (`src/websocket/`)
Contains WebSocket functionality:
- **handler.rs**: WebSocket connection management and message handling
- Implements echo server functionality as an example
- Separated from HTTP API logic for better organization

### Documentation (`src/docs/`)
Contains OpenAPI documentation generation:
- **mod.rs**: Defines OpenAPI schema and documentation structure
- Uses utoipa to generate Swagger/OpenAPI documentation
- Documentation functions are marked with `#[allow(dead_code)]` as they're used by macros

### Main (`src/main.rs`)
Application entry point:
- Server configuration and startup
- Combines HTTP API routes with WebSocket routes
- Integrates Swagger UI
- Minimal, focused on application orchestration

## Benefits of This Structure

1. **Separation of Concerns**: Each module has a clear, single responsibility
2. **Maintainability**: Easy to locate and modify specific functionality
3. **Testability**: Individual modules can be tested in isolation
4. **Scalability**: Easy to add new endpoints, models, or documentation
5. **Code Reuse**: Models and handlers can be reused across different routes

## Adding New Features

### Adding New API Endpoints

To add a new HTTP API endpoint:

1. Create the request/response models in `src/models/`
2. Implement the handler in `src/handlers/`
3. Add the route in `src/routes/api.rs`
4. Add documentation in `src/docs/mod.rs`
5. Update the module exports in the respective `mod.rs` files

### Adding WebSocket Features

To extend WebSocket functionality:

1. Modify or add new functions in `src/websocket/handler.rs`
2. Consider creating separate handlers for different WebSocket message types
3. Add any WebSocket-related models to `src/models/` if needed

## Running the Application

```bash
cargo run
```

The server will start on `http://localhost:3000` with:
- API endpoints available under `/api/`
- Swagger UI available at `/swagger-ui`