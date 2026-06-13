use crate::agenda_cultural::model::Category;
use lazy_static::lazy_static;
use opentelemetry::metrics::{Counter, Gauge, Histogram};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::{runtime, Resource};
use std::fmt::{Display, Formatter};
use std::time::Duration;
use tracing::warn;

#[derive(Clone, Copy, Debug)]
pub enum MetricResult {
    Ok,
    Error,
}

#[derive(Clone, Copy, Debug)]
pub enum PipelineStage {
    FetchEvents,
    SendEvents,
    BackupVotes,
}

#[derive(Clone, Copy, Debug)]
pub enum PipelineErrorKind {
    Api,
    Io,
    Serialize,
    EmptyResult,
}

impl Display for MetricResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            MetricResult::Ok => "ok",
            MetricResult::Error => "error",
        };
        write!(f, "{}", value)
    }
}

impl Display for PipelineStage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            PipelineStage::FetchEvents => "fetch_events",
            PipelineStage::SendEvents => "send_events",
            PipelineStage::BackupVotes => "backup_votes",
        };
        write!(f, "{}", value)
    }
}

impl Display for PipelineErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            PipelineErrorKind::Api => "api",
            PipelineErrorKind::Io => "io",
            PipelineErrorKind::Serialize => "serialize",
            PipelineErrorKind::EmptyResult => "empty_result",
        };
        write!(f, "{}", value)
    }
}

impl From<&Category> for KeyValue {
    fn from(category: &Category) -> Self {
        KeyValue::new("category", category.to_string().to_lowercase())
    }
}

impl From<MetricResult> for KeyValue {
    fn from(result: MetricResult) -> Self {
        KeyValue::new("result", result.to_string())
    }
}

impl From<PipelineStage> for KeyValue {
    fn from(stage: PipelineStage) -> Self {
        KeyValue::new("stage", stage.to_string())
    }
}

impl From<PipelineErrorKind> for KeyValue {
    fn from(error_kind: PipelineErrorKind) -> Self {
        KeyValue::new("error_kind", error_kind.to_string())
    }
}

pub fn setup_metrics(endpoint: &str) -> Option<SdkMeterProvider> {
    let meter_provider = opentelemetry_otlp::new_pipeline()
        .metrics(runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            "alertaemcena",
        )]))
        .with_period(Duration::from_secs(15))
        .build();

    match meter_provider {
        Ok(provider) => {
            global::set_meter_provider(provider.clone());
            Some(provider)
        }
        Err(e) => {
            warn!("Failed to build OTLP metrics exporter: {}", e);
            None
        }
    }
}

lazy_static! {
    static ref METER: opentelemetry::metrics::Meter =
        global::meter_provider().meter("alertaemcena");
    static ref EVENTS_FETCHED_TOTAL: Counter<u64> = METER
        .u64_counter("aec_events_fetched_total")
        .with_description("Total number of fetched events from agenda source")
        .init();
    static ref EVENTS_SENT_TOTAL: Counter<u64> = METER
        .u64_counter("aec_events_sent_total")
        .with_description("Total number of event send attempts")
        .init();
    static ref EVENT_SEND_DURATION_SECONDS: Histogram<f64> = METER
        .f64_histogram("aec_event_send_duration_seconds")
        .with_description("Duration of sending one event to Discord")
        .with_unit("s")
        .init();
    static ref REACTION_PROCESSING_DURATION_SECONDS: Histogram<f64> = METER
        .f64_histogram("aec_reaction_processing_duration_seconds")
        .with_description("Duration of reaction processing phase")
        .with_unit("s")
        .init();
    static ref DM_REVIEW_SENT_TOTAL: Counter<u64> = METER
        .u64_counter("aec_dm_review_sent_total")
        .with_description("Total DM review send attempts")
        .init();
    static ref VOTE_BACKUP_RECORDS_TOTAL: Counter<u64> = METER
        .u64_counter("aec_vote_backup_records_total")
        .with_description("Total vote records written to backups")
        .init();
    static ref VOTE_BACKUP_DURATION_SECONDS: Histogram<f64> = METER
        .f64_histogram("aec_vote_backup_duration_seconds")
        .with_description("Duration of vote backup phase")
        .with_unit("s")
        .init();
    static ref PIPELINE_RUN_DURATION_SECONDS: Histogram<f64> = METER
        .f64_histogram("aec_pipeline_run_duration_seconds")
        .with_description("Duration of one category pipeline run")
        .with_unit("s")
        .init();
    static ref PIPELINE_ERRORS_TOTAL: Counter<u64> = METER
        .u64_counter("aec_pipeline_errors_total")
        .with_description("Total pipeline errors")
        .init();
    static ref THREADS_ACTIVE: Gauge<u64> = METER
        .u64_gauge("aec_threads_active")
        .with_description("Current active thread count per category")
        .init();
}

pub fn record_events_fetched(category: &Category, count: u64) {
    EVENTS_FETCHED_TOTAL.add(count, &[category.into()]);
}

pub fn record_event_sent(category: &Category, result: MetricResult) {
    EVENTS_SENT_TOTAL.add(1, &[category.into(), result.into()]);
}

pub fn record_event_send_duration(category: &Category, duration: Duration) {
    EVENT_SEND_DURATION_SECONDS.record(duration.as_secs_f64(), &[category.into()]);
}

pub fn record_reaction_processing_duration(category: &Category, duration: Duration) {
    REACTION_PROCESSING_DURATION_SECONDS.record(duration.as_secs_f64(), &[category.into()]);
}

pub fn record_dm_review_sent(result: MetricResult) {
    DM_REVIEW_SENT_TOTAL.add(1, &[result.into()]);
}

pub fn record_vote_backup_records(count: u64) {
    VOTE_BACKUP_RECORDS_TOTAL.add(count, &[]);
}

pub fn record_vote_backup_duration(result: MetricResult, duration: Duration) {
    VOTE_BACKUP_DURATION_SECONDS.record(duration.as_secs_f64(), &[result.into()]);
}

pub fn record_pipeline_run_duration_without_event_gather(category: &Category, duration: Duration) {
    PIPELINE_RUN_DURATION_SECONDS.record(
        duration.as_secs_f64(),
        &[category.into(), KeyValue::new("gathered_events", false)],
    );
}

pub fn record_pipeline_run_duration(category: &Category, duration: Duration) {
    PIPELINE_RUN_DURATION_SECONDS.record(
        duration.as_secs_f64(),
        &[category.into(), KeyValue::new("gathered_events", true)],
    );
}

pub fn record_pipeline_error(stage: PipelineStage, error_kind: PipelineErrorKind) {
    PIPELINE_ERRORS_TOTAL.add(1, &[stage.into(), error_kind.into()]);
}

pub fn set_threads_active(category: &Category, count: u64) {
    THREADS_ACTIVE.record(count, &[category.into()]);
}
