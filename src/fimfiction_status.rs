use chrono::{DateTime, Utc};

/// Fimfiction status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FimficStatus {
	Available,
	Unreachable,
}

/// Fimfiction status data for uptime
#[derive(Debug, Clone)]
pub(crate) struct FimficStatusData {
	pub(crate) datetime: DateTime<Utc>,
	pub(crate) api_duration: Option<i32>,
	pub(crate) round_trip: Option<i32>,
}

impl From<DateTime<Utc>> for FimficStatusData {
	fn from(value: DateTime<Utc>) -> Self {
		Self {
			datetime: value,
			api_duration: None,
			round_trip: None,
		}
	}
}
