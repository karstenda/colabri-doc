# colabri-doc

A Rust server with WebSocket support, REST API, and automatically generated Swagger documentation.

## Features

- **WebSocket Server**: Real-time bidirectional communication at `/ws`
- **REST API**: HTTP endpoints under `/api` route
- **Swagger Documentation**: Auto-generated OpenAPI documentation at `/swagger`

## Getting Started

### Configuration

The application can be configured using environment variables or an `app.env` file. If an `app.env` file exists, it will be loaded automatically. Otherwise, the application will look for a standard `.env` file or use environment variables directly.

#### Setting up Configuration

In order to develop locally, we have already created an `app.env` file on Google Cloud Secret Manager. Pull it from there by running:

Linux

```bash
gcloud secrets versions access latest --secret="colabri-doc_app_env" --format='get(payload.data)' | tr '_-' '/+' | base64 -d > app.env
```

Windows

```bash
(gcloud secrets versions access latest --secret="colabri-doc_app_env" --format='get(payload.data)') -replace '_', '/' -replace '-', '+' | ForEach-Object { [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String($_)) } | Out-File app.env -Encoding UTF8
```
