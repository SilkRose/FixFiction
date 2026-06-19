use super::Db;
use crate::chapter::Chapter;
use crate::error::{Result, db_insert_err, db_select_err};
use sqlx::postgres::PgQueryResult;

impl Db {
	/// Selects a [Chapter] from the database, based on Story ID and Chapter number
	pub(crate) async fn get_story_chapter(
		&self, story_id: i32, chapter_num: i32,
	) -> Result<Option<Chapter>> {
		sqlx::query_as!(
			Chapter,
			r#"SELECT
				id, story_id, chapter_num, title, link, views,
				words, date_published, date_modified, date_cached
			FROM Chapters
			WHERE
				story_id = $1
			AND
				chapter_num = $2
			LIMIT 1;"#,
			story_id,
			chapter_num
		)
		.fetch_optional(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Selects a [Chapter] from the database, based on Chapter ID
	pub(crate) async fn get_chapter(&self, id: i32) -> Result<Option<Chapter>> {
		sqlx::query_as!(
			Chapter,
			r#"SELECT
				id, story_id, chapter_num, title, link, views,
				words, date_published, date_modified, date_cached
			FROM Chapters
			WHERE id = $1
			LIMIT 1;"#,
			id
		)
		.fetch_optional(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Inserts a [Chapter] into the database
	pub(crate) async fn insert_chapter(&self, data: &Chapter) -> Result<PgQueryResult> {
		sqlx::query!(
			r#"INSERT INTO Chapters 
				(id, story_id, chapter_num, title, link, views,
				words, date_published, date_modified, date_cached)
			VALUES
				($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
			ON CONFLICT(id) DO UPDATE SET
				story_id = EXCLUDED.story_id,
				chapter_num = EXCLUDED.chapter_num,
				title = EXCLUDED.title,
				link = EXCLUDED.link,
				views = EXCLUDED.views,
				words = EXCLUDED.words,
				date_published = EXCLUDED.date_published,
				date_modified = EXCLUDED.date_modified,
				date_cached = EXCLUDED.date_cached;"#,
			data.id,
			data.story_id,
			data.chapter_num,
			data.title,
			data.link,
			data.views,
			data.words,
			data.date_published,
			data.date_modified,
			data.date_cached,
		)
		.execute(&self.pool)
		.await
		.map_err(db_insert_err)
	}
}
