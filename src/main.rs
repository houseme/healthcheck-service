use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::get,
};
use once_cell::sync::Lazy;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics;
use opentelemetry_sdk::metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider};
use opentelemetry_semantic_conventions::{
    SCHEMA_URL,
    attribute::{
        DEPLOYMENT_ENVIRONMENT_NAME, NETWORK_LOCAL_ADDRESS, SERVICE_NAME, SERVICE_VERSION,
    },
};
use prometheus::{Encoder, Registry, TextEncoder};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Mutex;
use sysinfo::System;
use tokio::time::{Duration, Instant, sleep};
use tracing::{info, warn};

#[derive(Clone)]
#[allow(dead_code)]
struct AppState {
    meter: opentelemetry::metrics::Meter,
}

///
static GLOBAL_REGISTRY: Lazy<Mutex<Registry>> = Lazy::new(|| Mutex::new(Registry::new()));
// 主程序入口
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let meter_provider = setup_meter_provider();
    global::set_meter_provider(meter_provider.clone());

    let meter = global::meter("healthcheck-service");
    let app_state = AppState { meter };

    tokio::spawn(update_service_status());
    tokio::spawn(update_system_metrics());

    let app = Router::new()
        .route("/health/live", get(liveness_probe))
        .route("/health/ready", get(readiness_probe))
        .route("/metrics", get(metrics_handler))
        .route("/api/example", get(api_example_handler)) // 示例 API 端点
        .route("/api/fail", get(api_fail_handler)) // 示例失败端点
        .with_state(app_state)
        .layer(middleware::from_fn(track_api_metrics));

    let addr = SocketAddr::from(([127, 0, 0, 1], 5000));
    info!("Server running at http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    // meter_provider.shutdown().unwrap();
}

// 配置 MeterProvider
fn setup_meter_provider() -> SdkMeterProvider {
    let service_name = "healthcheck-service";
    let resource = opentelemetry_sdk::Resource::builder()
        .with_service_name(service_name)
        .with_schema_url(
            [
                KeyValue::new(SERVICE_NAME, service_name),
                KeyValue::new(SERVICE_VERSION, "0.1.0"),
                KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, "development"),
                KeyValue::new(NETWORK_LOCAL_ADDRESS, "127.0.0.1"),
            ],
            SCHEMA_URL,
        )
        .build();

    let otlp_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:4317")
        .with_temporality(metrics::Temporality::default())
        .build()
        .unwrap();

    let otlp_reader = PeriodicReader::builder(otlp_exporter)
        .with_interval(Duration::from_secs(60))
        .build();

    // Get a reference to the registry for reading metrics
    let registry = GLOBAL_REGISTRY.lock().unwrap().to_owned();
    let prometheus_exporter = opentelemetry_prometheus::exporter()
        .with_registry(registry)
        .build()
        .unwrap();

    MeterProviderBuilder::default()
        .with_resource(resource)
        .with_reader(otlp_reader)
        .with_reader(prometheus_exporter)
        .build()
}

// 更新服务状态指标
async fn update_service_status() {
    let meter = global::meter("healthcheck-service");
    let up_counter = meter.u64_counter("service.up").build();

    let mut is_ready = true;
    loop {
        up_counter.add(1, &[KeyValue::new("status", "alive")]);
        is_ready = !is_ready;
        meter
            .u64_observable_gauge("service.ready")
            .with_callback(move |observer| {
                observer.observe(
                    if is_ready { 1 } else { 0 },
                    &[KeyValue::new("status", "ready")],
                );
            })
            .build();
        sleep(Duration::from_secs(10)).await;
    }
}

// 更新系统指标
async fn update_system_metrics() {
    let meter = global::meter("healthcheck-service");
    let mut system = System::new_all();

    loop {
        system.refresh_all();
        let cpu_usage = system.global_cpu_info().cpu_usage() as f64 / 100.0;
        meter
            .f64_observable_gauge("system_cpu_usage")
            .with_callback(move |observer| {
                observer.observe(cpu_usage, &[]);
            })
            .build();

        let mem_used = system.used_memory();
        meter
            .u64_observable_gauge("system_mem_used")
            .with_callback(move |observer| {
                observer.observe(mem_used, &[]);
            })
            .build();
        sleep(Duration::from_secs(5)).await;
    }
}

// API 指标中间件
async fn track_api_metrics(req: Request<Body>, next: Next) -> Response {
    let start_time = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    let response = next.run(req).await;
    let status = response.status().as_u16().to_string();
    let duration = start_time.elapsed().as_secs_f64();

    // Get meter from global provider
    let meter = global::meter("healthcheck-service");
    let request_counter = meter.u64_counter("api_requests_total").build();
    let request_duration = meter.f64_histogram("api_request_duration_seconds").build();
    let error_counter = meter.u64_counter("api_errors_total").build();

    let attributes = &[
        KeyValue::new("method", method),
        KeyValue::new("path", path),
        KeyValue::new("status", status),
    ];

    request_counter.add(1, attributes);
    request_duration.record(duration, attributes);

    if response.status().is_server_error() || response.status().is_client_error() {
        error_counter.add(1, attributes);
    }

    response
}

// 存活性检查端点
async fn liveness_probe() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "message": "Service is alive"
    }))
}

// 就绪性检查端点
async fn readiness_probe() -> Json<serde_json::Value> {
    let meter = global::meter("healthcheck-service");
    let is_ready = 1;
    meter
        .u64_observable_gauge("service.ready")
        .with_callback(move |observer| {
            observer.observe(is_ready, &[]);
        })
        .build();
    Json(json!({
        "status": if is_ready == 1 { "ok" } else { "not_ready" },
        "message": if is_ready == 1 { "Service is ready" } else { "Service is not ready" }
    }))
}

// 示例 API 端点
async fn api_example_handler() -> impl IntoResponse {
    Json(json!({
        "message": "API example response"
    }))
}

// 示例失败端点
async fn api_fail_handler() -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
}

// Prometheus 指标端点
async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    // Get a reference to the registry for reading metrics
    let registry = GLOBAL_REGISTRY.lock().unwrap().to_owned();
    let metric_families = registry.gather();
    if metric_families.is_empty() {
        warn!("No metrics available in Prometheus registry");
    } else {
        info!("Metrics collected: {} families", metric_families.len());
    }
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap_or_else(|_| "Error encoding metrics".to_string())
}
