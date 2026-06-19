use super::Db;
use crate::error::{Result, db_insert_err, db_select_err};
use crate::tag::{Tag, TagType};
use sqlx::postgres::PgQueryResult;

impl Db {
	/// Selects a [Tag] from the database
	pub(crate) async fn get_tag(&self, id: i32) -> Result<Option<Tag>> {
		sqlx::query_as!(
			Tag,
			r#"SELECT
				id, name, type AS "tag_type: TagType", old_id, link, date_cached
			FROM Tags
			WHERE id = $1
			LIMIT 1;"#,
			id
		)
		.fetch_optional(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Inserts a [Tag] into the database
	pub(crate) async fn insert_tag(&self, tag: &Tag) -> Result<PgQueryResult> {
		sqlx::query!(
			r#"INSERT INTO Tags
				(id, name, type, old_id, link, date_cached)
			VALUES
				($1, $2, $3, $4, $5, $6)
			ON CONFLICT(id) DO UPDATE SET
				name = EXCLUDED.name,
				type = EXCLUDED.type,
				old_id = EXCLUDED.old_id,
				link = EXCLUDED.link,
				date_cached = EXCLUDED.date_cached;"#,
			tag.id,
			tag.name,
			tag.tag_type as _,
			tag.old_id,
			tag.link,
			tag.date_cached,
		)
		.execute(&self.pool)
		.await
		.map_err(db_insert_err)
	}
}
