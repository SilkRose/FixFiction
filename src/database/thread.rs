use super::Db;
use crate::error::{Result, db_insert_err, db_select_err};
use crate::thread::Thread;
use sqlx::postgres::PgQueryResult;

impl Db {
	/// Selects a [Thread] from the database
	pub(crate) async fn get_thread(&self, id: i32) -> Result<Option<Thread>> {
		sqlx::query_as!(
			Thread,
			r#"SELECT
				id, group_id, creator_id, last_poster_id, title, link, posts,
				sticky, locked, date_created, date_last_post, date_cached
			FROM Threads
			WHERE id = $1
			LIMIT 1;"#,
			id
		)
		.fetch_optional(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Inserts a [Thread] into the database
	pub(crate) async fn insert_thread(&self, data: &Thread) -> Result<PgQueryResult> {
		sqlx::query!(
			r#"INSERT INTO Threads 
				(id, group_id, creator_id, last_poster_id, title, link, posts,
				sticky, locked, date_created, date_last_post, date_cached)
			VALUES
				($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
			ON CONFLICT(id) DO UPDATE SET
				group_id = EXCLUDED.group_id,
				creator_id = EXCLUDED.creator_id,
				last_poster_id = EXCLUDED.last_poster_id,
				title = EXCLUDED.title,
				link = EXCLUDED.link,
				posts = EXCLUDED.posts,
				sticky = EXCLUDED.sticky,
				locked = EXCLUDED.locked,
				date_created = EXCLUDED.date_created,
				date_last_post = EXCLUDED.date_last_post,
				date_cached = EXCLUDED.date_cached;"#,
			data.id,
			data.group_id,
			data.creator_id,
			data.last_poster_id,
			data.title,
			data.link,
			data.posts,
			data.sticky,
			data.locked,
			data.date_created,
			data.date_last_post,
			data.date_cached,
		)
		.execute(&self.pool)
		.await
		.map_err(db_insert_err)
	}
}
