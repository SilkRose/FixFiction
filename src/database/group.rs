use super::Db;
use crate::error::{Result, db_insert_err, db_select_err};
use crate::group::Group;
use sqlx::postgres::PgQueryResult;

impl Db {
	/// Selects a [Group] from the database
	pub(crate) async fn get_group(&self, id: i32) -> Result<Option<Group>> {
		sqlx::query_as!(
			Group,
			"SELECT
				id, name, description, link, members,
				stories, founder_id, icon_url, nsfw,
				open, hidden, date_created, date_cached
			FROM Groups
			WHERE id = $1
			LIMIT 1;",
			id
		)
		.fetch_optional(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Inserts a [Group] into the database
	pub(crate) async fn insert_group(&self, data: &Group) -> Result<PgQueryResult> {
		sqlx::query!(
			"INSERT INTO Groups
				(id, name, description, link, members,
				stories, founder_id, icon_url, nsfw,
				open, hidden, date_created, date_cached)
			VALUES
				($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
			ON CONFLICT(id) DO UPDATE SET
				name = EXCLUDED.name,
				description = EXCLUDED.description,
				link = EXCLUDED.link,
				members = EXCLUDED.members,
				stories = EXCLUDED.stories,
				founder_id = EXCLUDED.founder_id,
				icon_url = EXCLUDED.icon_url,
				nsfw = EXCLUDED.nsfw,
				open = EXCLUDED.open,
				hidden = EXCLUDED.hidden,
				date_created = EXCLUDED.date_created,
				date_cached = EXCLUDED.date_cached;",
			data.id,
			data.name,
			data.description,
			data.link,
			data.members,
			data.stories,
			data.founder_id,
			data.icon_url,
			data.nsfw,
			data.open,
			data.hidden,
			data.date_created,
			data.date_cached,
		)
		.execute(&self.pool)
		.await
		.map_err(db_insert_err)
	}
}
