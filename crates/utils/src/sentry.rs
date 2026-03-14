use sentry_tracing::{EventFilter, SentryLayer};

#[derive(Clone, Copy, Debug)]
pub enum SentrySource {
    Backend,
    Mcp,
    Remote,
}

/// Telemetry disabled — this is a no-op.
pub fn init_once(_source: SentrySource) {}

/// Telemetry disabled — this is a no-op.
pub fn configure_user_scope(_user_id: &str, _username: Option<&str>, _email: Option<&str>) {}

/// Returns a no-op Sentry tracing layer (ignores all events).
pub fn sentry_layer<S>() -> SentryLayer<S>
where
    S: tracing::Subscriber,
    S: for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    SentryLayer::default()
        .event_filter(|_meta| EventFilter::Ignore)
}
