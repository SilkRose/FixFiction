use super::Db;
use crate::error::{Result, db_insert_err, db_select_err};
use crate::fimfiction_status::FimficStatusData;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgQueryResult;

impl Db {
	/// Selects the latest count of [FimficStatusData]
	pub(crate) async fn get_last_n_status_minutes(
		&self, count: i64,
	) -> Result<Vec<FimficStatusData>> {
		sqlx::query_as!(
			FimficStatusData,
			r#"SELECT
				datetime, api_duration, round_trip, challenged
			FROM Fimfic_status_minutes
			ORDER BY datetime DESC
			limit $1;"#,
			count
		)
		.fetch_all(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Selects all [FimficStatusData] in a given date range
	pub(crate) async fn get_status_in_range_minutes(
		&self, start: &DateTime<Utc>, end: &DateTime<Utc>,
	) -> Result<Vec<FimficStatusData>> {
		sqlx::query_as!(
			FimficStatusData,
			r#"SELECT
				datetime, api_duration, round_trip, challenged
			FROM Fimfic_status_minutes
			WHERE
				datetime >= $1
			AND
				datetime <= $2;"#,
			start,
			end
		)
		.fetch_all(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Inserts a [FimficStatusData] into the database
	pub(crate) async fn insert_minute_status(
		&self, data: &FimficStatusData,
	) -> Result<PgQueryResult> {
		sqlx::query!(
			r#"INSERT INTO Fimfic_status_minutes
				(datetime, api_duration, round_trip, challenged)
			VALUES
				($1, $2, $3, $4);"#,
			data.datetime,
			data.api_duration,
			data.round_trip,
			data.challenged
		)
		.execute(&self.pool)
		.await
		.map_err(db_insert_err)
	}
}
