use super::Db;
use crate::bookshelf::Bookshelf;
use crate::error::{Result, db_insert_err, db_select_err};
use sqlx::postgres::PgQueryResult;

impl Db {
	/// Selects a [Bookshelf] from the database
	pub(crate) async fn get_bookshelf(&self, id: i32) -> Result<Option<Bookshelf>> {
		sqlx::query_as!(
			Bookshelf,
			"SELECT
				id, name, description, link, color, icon_url, stories,
				num_unread, track_unread, quick_add, email_update,
				user_id, order_pos, date_created, date_modified, date_cached
			FROM Bookshelves
			WHERE id = $1
			LIMIT 1;",
			id
		)
		.fetch_optional(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Inserts a [Bookshelf] into the database
	pub(crate) async fn insert_bookshelf(&self, data: &Bookshelf) -> Result<PgQueryResult> {
		sqlx::query!(
			"INSERT INTO Bookshelves
				(id, name, description, link, color, icon_url, stories,
				num_unread, track_unread, quick_add, email_update, user_id,
				order_pos, date_created, date_modified, date_cached)
			VALUES
				($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
			ON CONFLICT(id) DO UPDATE SET
				name = EXCLUDED.name,
				description = EXCLUDED.description,
				link = EXCLUDED.link,
				color = EXCLUDED.color,
				icon_url = EXCLUDED.icon_url,
				stories = EXCLUDED.stories,
				num_unread = EXCLUDED.num_unread,
				track_unread = EXCLUDED.track_unread,
				quick_add = EXCLUDED.quick_add,
				email_update = EXCLUDED.email_update,
				user_id = EXCLUDED.user_id,
				order_pos = EXCLUDED.order_pos,
				date_created = EXCLUDED.date_created,
				date_modified = EXCLUDED.date_modified,
				date_cached = EXCLUDED.date_cached;",
			data.id,
			data.name,
			data.description,
			data.link,
			data.color,
			data.icon_url,
			data.stories,
			data.num_unread,
			data.track_unread,
			data.quick_add,
			data.email_update,
			data.user_id,
			data.order_pos,
			data.date_created,
			data.date_modified,
			data.date_cached,
		)
		.execute(&self.pool)
		.await
		.map_err(db_insert_err)
	}
}
