use lazy_static::lazy_static;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
use std::str::FromStr;
use std::time::Duration;
use std::{env, io};
use tokio::task::JoinHandle;
use tracing::{error, info, warn, Level};
use tracing_loki::url::Url;
use tracing_loki::{BackgroundTask, BackgroundTaskController};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{filter, fmt};

lazy_static! {
    static ref LOKI_URL: Option<String> = env::var("LOKI_URL").ok();
    // gRPC endpoint for Grafana Alloy (default: http://localhost:4317)
    static ref OTLP_ENDPOINT: Option<String> = env::var("OTLP_ENDPOINT").ok();
}

pub struct TracingHandles {
    pub loki: Option<(BackgroundTaskController, JoinHandle<()>)>,
    // Held so shutdown() can be called while the Tokio runtime is still alive.
    // If dropped without calling shutdown(), the batch exporter may lose buffered spans.
    pub otel_provider: Option<SdkTracerProvider>,
}

impl TracingHandles {
    pub async fn shutdown(self) {
        if let Some(provider) = self.otel_provider {
            if let Err(e) = provider.shutdown() {
                error!("Failed to flush OTel traces: {}", e);
            }
        }

        if let Some((controller, handle)) = self.loki {
            controller.shutdown().await;
            handle.await.expect("Failed joining Loki task");
        }
    }
}

fn build_loki_layer(
    base_url: Url,
) -> (
    tracing_loki::Layer,
    BackgroundTaskController,
    BackgroundTask,
) {
    tracing_loki::builder()
        .label("service", "alertaemcena")
        .expect("Failed setting label")
        .build_controller_url(base_url)
        .unwrap()
}

// OTel layer must be added FIRST (.with() closest to Registry) because
// OpenTelemetryLayer<Registry, T> only satisfies Layer<Registry>, not Layer<Layered<...>>.
// Subsequent layers (filter, fmt, loki) are generic over S and stack on top cleanly.
fn build_otlp(
    endpoint: &str,
) -> Option<(
    tracing_opentelemetry::OpenTelemetryLayer<
        tracing_subscriber::Registry,
        opentelemetry_sdk::trace::Tracer,
    >,
    SdkTracerProvider,
)> {
    let result = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .with_trace_config(opentelemetry_sdk::trace::Config::default().with_resource(
            opentelemetry_sdk::Resource::new(vec![opentelemetry::KeyValue::new(
                "service.name",
                "alertaemcena",
            )]),
        ))
        .install_batch(runtime::Tokio);

    match result {
        Ok(provider) => {
            let tracer = provider.tracer("alertaemcena");
            let layer = tracing_opentelemetry::layer().with_tracer(tracer);
            Some((layer, provider))
        }
        Err(e) => {
            warn!(
                "Failed to build OTLP exporter: {}. Continuing without it.",
                e
            );
            None
        }
    }
}

async fn is_loki_reachable(url: &Url) -> bool {
    reqwest::Client::new()
        .head(url.clone())
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .is_ok()
}

pub async fn setup_tracing() -> TracingHandles {
    let filter = filter::Targets::new()
        .with_target(
            "alertaemcena",
            Level::from_str(&env::var("APP_LOG_LEVEL")
                .unwrap_or_else(|_| "debug".to_string()))
                .expect("Unknown log level provided"),
        )
        .with_default(Level::WARN);

    let loki_url = LOKI_URL
        .as_ref()
        .map(|url| url.parse::<Url>().expect("Invalid Loki URL format"));

    let loki_reachable = match &loki_url {
        Some(url) => is_loki_reachable(url).await,
        None => false,
    };

    if loki_url.is_some() && !loki_reachable {
        warn!("Couldn't connect to Loki. Continuing without it.");
    } else if loki_url.is_none() {
        warn!("Loki URL not provided. Continuing without it.");
    }

    let (otel_layer, otel_provider): (
        Option<
            tracing_opentelemetry::OpenTelemetryLayer<
                tracing_subscriber::Registry,
                opentelemetry_sdk::trace::Tracer,
            >,
        >,
        Option<SdkTracerProvider>,
    ) = match OTLP_ENDPOINT.as_deref().and_then(build_otlp) {
        Some((layer, provider)) => (Some(layer), Some(provider)),
        None => (None, None),
    };

    let loki_handle = loki_url.filter(|_| loki_reachable).map(build_loki_layer);

    // Option<L> implements Layer<S> when L: Layer<S>, so these are no-op when None.
    // OTel layer goes innermost (first .with()) — only implements Layer<Registry>, not Layer<Layered<...>>.
    match loki_handle {
        Some((loki_layer, controller, task)) => {
            tracing_subscriber::registry()
                .with(otel_layer)
                .with(filter)
                .with(fmt::layer().with_writer(io::stdout))
                .with(loki_layer)
                .init();
            let handle = tokio::spawn(task);

            if otel_provider.is_some() {
                info!("Loki + OTLP initialized");
            } else {
                info!("Loki initialized");
            }

            TracingHandles {
                loki: Some((controller, handle)),
                otel_provider,
            }
        }
        None => {
            tracing_subscriber::registry()
                .with(otel_layer)
                .with(filter)
                .with(fmt::layer().with_writer(io::stdout))
                .init();

            if otel_provider.is_some() {
                info!("OTLP initialized");
            }

            TracingHandles {
                loki: None,
                otel_provider,
            }
        }
    }
}
