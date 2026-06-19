use super::Db;
use crate::blog::Blog;
use crate::error::{Result, db_insert_err, db_select_err};
use sqlx::postgres::PgQueryResult;

impl Db {
	/// Selects a [Blog] from the database
	pub(crate) async fn get_blog(&self, id: i32) -> Result<Option<Blog>> {
		sqlx::query_as!(
			Blog,
			"SELECT
				id, title, content, link, comments, views,
				author_id, tags, story_id, date_posted, date_cached
			FROM Blogs
			WHERE id = $1
			LIMIT 1;",
			id
		)
		.fetch_optional(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Inserts a [Blog] into the database
	pub(crate) async fn insert_blog(&self, blog: &Blog) -> Result<PgQueryResult> {
		sqlx::query!(
			"INSERT INTO Blogs 
				(id, title, content, link, comments, views,
				author_id, tags, story_id, date_posted, date_cached)
			VALUES
				($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
			ON CONFLICT(id) DO UPDATE SET
				title = EXCLUDED.title,
				content = EXCLUDED.content,
				link = EXCLUDED.link,
				comments = EXCLUDED.comments,
				views = EXCLUDED.views,
				author_id = EXCLUDED.author_id,
				tags = EXCLUDED.tags,
				story_id = EXCLUDED.story_id,
				date_posted = EXCLUDED.date_posted,
				date_cached = EXCLUDED.date_cached;",
			blog.id,
			blog.title,
			blog.content,
			blog.link,
			blog.comments,
			blog.views,
			blog.author_id,
			blog.tags,
			blog.story_id,
			blog.date_posted,
			blog.date_cached,
		)
		.execute(&self.pool)
		.await
		.map_err(db_insert_err)
	}
}
