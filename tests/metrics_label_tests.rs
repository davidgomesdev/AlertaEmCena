use alertaemcena::agenda_cultural::model::Category;
use alertaemcena::metrics::{MetricResult, PipelineErrorKind, PipelineStage};
use opentelemetry::KeyValue;

#[test]
fn should_convert_metric_enums_to_labels_using_to_string() {
    assert_eq!(MetricResult::Ok.to_string(), "ok");
    assert_eq!(MetricResult::Error.to_string(), "error");

    assert_eq!(PipelineStage::FetchEvents.to_string(), "fetch_events");
    assert_eq!(PipelineStage::SendEvents.to_string(), "send_events");

    assert_eq!(PipelineErrorKind::Api.to_string(), "api");
    assert_eq!(PipelineErrorKind::Io.to_string(), "io");
}

#[test]
fn should_convert_metric_dimensions_into_key_value() {
    let category_kv: KeyValue = (&Category::Teatro).into();
    let result_kv: KeyValue = MetricResult::Error.into();
    let stage_kv: KeyValue = PipelineStage::BackupVotes.into();
    let error_kv: KeyValue = PipelineErrorKind::Serialize.into();

    assert_eq!(category_kv.key.as_str(), "category");
    assert_eq!(category_kv.value.to_string(), "teatro");

    assert_eq!(result_kv.key.as_str(), "result");
    assert_eq!(result_kv.value.to_string(), "error");

    assert_eq!(stage_kv.key.as_str(), "stage");
    assert_eq!(stage_kv.value.to_string(), "backup_votes");

    assert_eq!(error_kv.key.as_str(), "error_kind");
    assert_eq!(error_kv.value.to_string(), "serialize");
}
