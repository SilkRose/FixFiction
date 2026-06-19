use super::Db;
use crate::error::{Result, db_delete_err, db_insert_err, db_select_err};
use crate::tag::TagLink;
use sqlx::postgres::PgQueryResult;

impl Db {
	/// Selects [TagLink]s from the database for a given story ID
	pub(crate) async fn get_tag_links(&self, story_id: i32) -> Result<Vec<TagLink>> {
		sqlx::query_as!(
			TagLink,
			r#"SELECT
				story_id, tag_id, date_cached
			FROM Tag_links
			WHERE story_id = $1;"#,
			story_id
		)
		.fetch_all(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Inserts a link between a [Story] and a [Tag] into the database
	pub(crate) async fn insert_tag_link(
		&self, story_id: i32, tag_id: i32,
	) -> Result<PgQueryResult> {
		sqlx::query_as!(
			TagLink,
			r#"INSERT INTO Tag_links
				(story_id, tag_id)
			VALUES
				($1, $2)
			ON CONFLICT(story_id, tag_id) DO UPDATE SET
				date_cached = now();"#,
			story_id,
			tag_id
		)
		.execute(&self.pool)
		.await
		.map_err(db_insert_err)
	}

	/// Deletes all tag links for a given [Story] ID
	pub(crate) async fn remove_tag_links(&self, story_id: i32) -> Result<PgQueryResult> {
		sqlx::query!("DELETE FROM Tag_links WHERE story_id = $1", story_id)
			.execute(&self.pool)
			.await
			.map_err(db_delete_err)
	}
}
