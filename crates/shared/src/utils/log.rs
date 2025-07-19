use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use tracing_appender::{
    non_blocking,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init_logger(sdk_logger_provider: SdkLoggerProvider, component: &str) {
    let log_file_name = format!("rust_app_{component}.log");

    let file_appender = RollingFileAppender::new(Rotation::DAILY, "/var/log/app", log_file_name);
    let (non_blocking, _guard) = non_blocking(file_appender);

    let filter_otel = EnvFilter::new("info")
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("opentelemetry=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());

    let otel_layer = OpenTelemetryTracingBridge::new(&sdk_logger_provider).with_filter(filter_otel);

    let filter_fmt = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info").add_directive("opentelemetry=debug".parse().unwrap())
    });

    let console_layer = fmt::layer()
        .with_thread_names(true)
        .with_ansi(true)
        .pretty()
        .with_filter(filter_fmt);

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .json();

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(console_layer)
        .with(file_layer)
        .init();

    std::mem::forget(_guard); // or store it globally
}
