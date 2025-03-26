use opentelemetry::KeyValue;
use opentelemetry::metrics::MeterProvider;
use opentelemetry_sdk::metrics::{MeterProviderBuilder, SdkMeterProvider};
use prometheus::{Encoder, Registry, TextEncoder};

struct ModuleMetrics {
    registry: Registry,
    provider: SdkMeterProvider,
}

impl ModuleMetrics {
    fn new(name: &str) -> Self {
        let registry = Registry::new();
        let exporter = opentelemetry_prometheus::exporter()
            .with_registry(registry.clone())
            .build()
            .unwrap();
        let provider = MeterProviderBuilder::default()
            .with_reader(exporter)
            .build();
        Self { registry, provider }
    }

    fn increment_counter(&self) {
        let meter = self.provider.meter("module");
        let counter = meter.u64_counter("tasks_total").build();
        counter.add(1, &[KeyValue::new("module", "background")]);
    }

    fn gather(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

fn main() {
    let api_metrics = ModuleMetrics::new("api");
    let bg_metrics = ModuleMetrics::new("background");

    api_metrics.increment_counter();
    bg_metrics.increment_counter();

    println!("API Metrics:\n{}", api_metrics.gather());
    println!("Background Metrics:\n{}", bg_metrics.gather());
}
