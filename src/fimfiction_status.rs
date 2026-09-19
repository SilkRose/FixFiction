use chrono::{DateTime, Timelike, Utc};

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
	pub(crate) challenged: bool,
}

impl From<DateTime<Utc>> for FimficStatusData {
	/// Converts a [DateTime<Utc>] into a [FimficStatusData]
	fn from(value: DateTime<Utc>) -> Self {
		Self {
			datetime: value,
			api_duration: None,
			round_trip: None,
			challenged: false,
		}
	}
}

impl FimficStatusData {
	/// Flattens the seconds and sub-seconds to zero
	pub fn flatten_seconds(mut self) -> Self {
		if let Some(date) = self.datetime.with_second(0)
			&& let Some(date) = date.with_nanosecond(0)
		{
			self.datetime = date;
		} else {
			eprintln!(
				"Fimfiction status zeroing seconds/sub-seconds failed: {}",
				self.datetime
			);
		}
		self
	}
}
