# Health Check Service

A Rust-based health monitoring microservice with built-in observability using OpenTelemetry and Prometheus for
comprehensive metrics collection and reporting.

## Features

- **Health Status Endpoints**: Liveness and readiness probes for Kubernetes compatibility
- **Metrics Collection**: Prometheus-compatible metrics endpoint
- **System Monitoring**: CPU and memory usage tracking
- **API Telemetry**: Request counts, durations, and error rates
- **OpenTelemetry Integration**: Support for distributed tracing and metrics export

## Getting Started

### Prerequisites

- Rust 2024 edition or newer
- Optional: Running Prometheus server for metrics collection
- Optional: OpenTelemetry collector for distributed tracing

### Installation

```bash
git clone https://github.com/houseme/healthcheck-service.git
cd healthcheck-service
cargo build --release
```

### Running the Service

```bash
cargo run
```

The service will start on http://127.0.0.1:5000

## API Endpoints

- **GET /health/live**: Liveness probe
- **GET /health/ready**: Readiness probe
- **GET /metrics**: Prometheus metrics endpoint
- **GET /api/example**: Example API endpoint
- **GET /api/fail**: Example failure endpoint (returns 500)

## Metrics Available

- **service.up**: Counter tracking service uptime
- **service.ready**: Gauge tracking readiness state
- **system_cpu_usage**: CPU usage as a fraction (0.0-1.0)
- **system_mem_used**: Memory usage in bytes
- **api_requests_total**: Total API requests with method, path, and status labels
- **api_request_duration_seconds**: Request duration histogram
- **api_errors_total**: Count of API errors by type

## Configuration

Configuration is currently hardcoded. The service exports metrics to:

- Prometheus endpoint at http://127.0.0.1:5000/metrics
- OpenTelemetry collector at http://localhost:4317 (gRPC)

## Development

```bash
# Run tests
cargo test

# Run with development features
cargo run --features dev
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.