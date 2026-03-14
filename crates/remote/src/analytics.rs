use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AnalyticsConfig {
    pub posthog_api_key: String,
    pub posthog_api_endpoint: String,
}

impl AnalyticsConfig {
    /// Telemetry disabled — always returns None.
    pub fn from_env() -> Option<Self> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct AnalyticsService;

impl AnalyticsService {
    pub fn new(_config: AnalyticsConfig) -> Self {
        Self
    }

    /// Telemetry disabled — this is a no-op.
    pub fn track(&self, _user_id: Uuid, _event_name: &str, _properties: Value) {}
}
