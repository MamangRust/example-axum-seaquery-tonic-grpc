use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use tracing_subscriber::{EnvFilter, prelude::*};

pub fn init_logger(sdk_logger_provider: SdkLoggerProvider) {
    let filter_otel = EnvFilter::new("info")
        .add_directive("hyper=off".parse().expect("valid"))
        .add_directive("opentelemetry=off".parse().expect("valid"))
        .add_directive("tonic=off".parse().expect("valid"))
        .add_directive("h2=off".parse().expect("valid"))
        .add_directive("reqwest=off".parse().expect("valid"));

    let otel_layer = OpenTelemetryTracingBridge::new(&sdk_logger_provider).with_filter(filter_otel);

    let filter_fmt = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info").add_directive("opentelemetry=debug".parse().expect("valid"))
    });

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_names(true)
        .with_ansi(true)
        .pretty()
        .with_filter(filter_fmt);

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(fmt_layer)
        .init();
}
