use axum::{
    body::Body,
    extract::State,
    http::{header::CONTENT_TYPE, StatusCode},
    response::{IntoResponse, Response},
};
use prometheus_client::{encoding::text::encode, metrics::{counter::Counter, family::Family, gauge::Gauge}};
use prometheus_client_derive_encode::{EncodeLabelSet, EncodeLabelValue};
use std::{
    fs, sync::Arc, time::{SystemTime, UNIX_EPOCH}
};
use prometheus_client::registry::Registry;
use sysinfo::System;


use crate::state::AppState;

fn get_thread_count(pid: usize) -> Option<i64> {
    let path = format!("/proc/{}/status", pid);
    if let Ok(contents) = fs::read_to_string(path) {
        for line in contents.lines() {
            if line.starts_with("Threads:") {
                if let Some(thread_count) = line.split_whitespace().nth(1) {
                    return thread_count.parse::<i64>().ok();
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub memory_alloc_bytes: Gauge,
    pub memory_sys_bytes: Gauge,
    pub available_memory: Counter,
    pub thread_usage: Gauge,
    pub total_cpu_usage: Counter,
    pub process_start_time: Gauge,
}

impl SystemMetrics {
    pub fn new() -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let metrics = Self {
            memory_alloc_bytes: Gauge::default(),
            memory_sys_bytes: Gauge::default(),
            available_memory: Counter::default(),
            thread_usage: Gauge::default(),
            total_cpu_usage: Counter::default(),
            process_start_time: Gauge::default(),
        };

        metrics.process_start_time.set(start_time as i64);
        metrics
    }

    pub fn register(&self, registry: &mut Registry) {
        registry.register(
            "process_memory_alloc_bytes",
            "Current memory allocation in bytes",
            self.memory_alloc_bytes.clone(),
        );

        registry.register(
            "process_memory_sys_bytes",
            "Total system memory in bytes",
            self.memory_sys_bytes.clone(),
        );

        registry.register(
            "process_memory_frees_total",
            "Total Available Memory",
            self.available_memory.clone(),
        );

        registry.register(
            "process_thread_total",
            "Thread total",
            self.thread_usage.clone(),
        );

        registry.register(
            "total_cpu_usage",
            "Total cpu usage",
            self.total_cpu_usage.clone(),
        );

        registry.register(
            "process_start_time_seconds",
            "Start time of the process since unix epoch in seconds",
            self.process_start_time.clone(),
        );
    }

    pub async fn update_metrics(&self) {
        let mut sys = System::new_all();
        sys.refresh_all();

        let pid = std::process::id() as usize;

        if let Some(process) = sys.process(sysinfo::Pid::from(pid)) {
            let current_memory = process.memory() as i64;
            self.memory_alloc_bytes.set(current_memory);
            self.memory_sys_bytes.set(process.virtual_memory() as i64);

            let available_memory = sys.available_memory() / 1_024;
            self.available_memory.inc_by(available_memory);

            let total_cpu_usage = sys.global_cpu_usage();
            self.total_cpu_usage.inc_by(total_cpu_usage as u64);

            if let Some(thread_count) = get_thread_count(pid) {
                self.thread_usage.set(thread_count);
            }
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct MethodLabels {
    pub method: Method,
}

#[derive(Debug, Clone)]
pub struct Metrics {
    pub requests: Family<MethodLabels, Counter>,
}

impl Metrics {
    pub fn inc_requests(&self, method: Method) {
        self.requests.get_or_create(&MethodLabels { method }).inc();
    }
}


pub async fn run_metrics_collector(system_metrics: Arc<SystemMetrics>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
    loop {
        interval.tick().await;
        system_metrics.update_metrics().await;
    }
}

pub async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut buffer = String::new();
    encode(&mut buffer, &state.registry).unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .header(
            CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )
        .body(Body::from(buffer))
        .unwrap()
}