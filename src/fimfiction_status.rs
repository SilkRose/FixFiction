use chrono::{DateTime, Utc};

/// Fimfiction status data for uptime
#[derive(Debug, Clone)]
pub(crate) struct FimficStatus {
	pub(crate) datetime: DateTime<Utc>,
	pub(crate) api_duration: Option<u32>,
	pub(crate) round_trip: Option<u32>,
}
