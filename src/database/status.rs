use super::Db;
use crate::error::{Result, db_insert_err, db_select_err};
use crate::fimfiction_status::FimficStatusData;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgQueryResult;

impl Db {
	/// Selects all [FimficStatusData] in a given date range
	pub(crate) async fn get_status_in_range(
		&self, start: &DateTime<Utc>, end: &DateTime<Utc>,
	) -> Result<Vec<FimficStatusData>> {
		sqlx::query_as!(
			FimficStatusData,
			r#"SELECT
				datetime, api_duration, round_trip
			FROM Fimfic_status
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
	pub(crate) async fn insert_status(&self, data: &FimficStatusData) -> Result<PgQueryResult> {
		sqlx::query!(
			r#"INSERT INTO Fimfic_status 
				(datetime, api_duration, round_trip)
			VALUES
				($1, $2, $3);"#,
			data.datetime,
			data.api_duration,
			data.round_trip
		)
		.execute(&self.pool)
		.await
		.map_err(db_insert_err)
	}
}
