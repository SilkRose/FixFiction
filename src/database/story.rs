use super::Db;
use crate::error::{Result, db_insert_err, db_select_err};
use crate::story::{CompletionStatus, ContentRating, Story};
use sqlx::postgres::PgQueryResult;

impl Db {
	/// Selects a [Story] from the database
	pub(crate) async fn get_story(&self, id: i32) -> Result<Option<Story>> {
		sqlx::query_as!(
			Story,
			r#"SELECT
				id, title, short_description, description, published, link, cover_url,
				color_hex, views, total_views, words, chapters, comments, rating,
				completion_status AS "completion_status: CompletionStatus",
				content_rating AS "content_rating: ContentRating",
				likes, dislikes, author_id, date_modified,
				date_updated, date_published, date_cached
			FROM Stories
			WHERE id = $1
			LIMIT 1;"#,
			id
		)
		.fetch_optional(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Inserts a [Story] into the database
	pub(crate) async fn insert_story(&self, data: &Story) -> Result<PgQueryResult> {
		sqlx::query!(
			r#"INSERT INTO Stories (
				id, title, short_description, description, published, link, cover_url,
				color_hex, views, total_views, words, chapters, comments, rating,
				completion_status, content_rating, likes, dislikes, author_id,
				date_modified, date_updated, date_published, date_cached)
			VALUES
				($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
				$14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
			ON CONFLICT(id) DO UPDATE SET
				title = EXCLUDED.title,
				short_description = EXCLUDED.short_description,
				description = EXCLUDED.description,
				published = EXCLUDED.published,
				link = EXCLUDED.link,
				cover_url = EXCLUDED.cover_url,
				color_hex = EXCLUDED.color_hex,
				views = EXCLUDED.views,
				total_views = EXCLUDED.total_views,
				words = EXCLUDED.words,
				chapters = EXCLUDED.chapters,
				comments = EXCLUDED.comments,
				rating = EXCLUDED.rating,
				completion_status = EXCLUDED.completion_status,
				content_rating = EXCLUDED.content_rating,
				likes = EXCLUDED.likes,
				dislikes = EXCLUDED.dislikes,
				author_id = EXCLUDED.author_id,
				date_modified = EXCLUDED.date_modified,
				date_updated = EXCLUDED.date_updated,
				date_published = EXCLUDED.date_published,
				date_cached = EXCLUDED.date_cached;"#,
			data.id,
			data.title,
			data.short_description,
			data.description,
			data.published,
			data.link,
			data.cover_url,
			data.color_hex,
			data.views,
			data.total_views,
			data.words,
			data.chapters,
			data.comments,
			data.rating,
			data.completion_status as _,
			data.content_rating as _,
			data.likes,
			data.dislikes,
			data.author_id,
			data.date_modified,
			data.date_updated,
			data.date_published,
			data.date_cached,
		)
		.execute(&self.pool)
		.await
		.map_err(db_insert_err)
	}
}
