//! Reading and writing changes to the database.

use crate::error::{Result, db_delete_err, db_insert_err, db_select_err};
use crate::tag::{Tag, TagLink, TagType};
use sqlx::postgres::{PgPoolOptions, PgQueryResult};
use sqlx::{Pool, Postgres};

mod blog;
mod bookshelf;
mod chapter;
mod group;
mod story;
mod thread;
mod user;

#[derive(Clone)]
pub(crate) struct Db {
	pub(crate) pool: Pool<Postgres>,
}

impl Db {
	/// Creates a new [Db] instance
	pub(crate) async fn new(database_url: &str) -> Result<Self> {
		let pool = PgPoolOptions::new()
			.max_connections(16)
			.connect(database_url)
			.await?;
		sqlx::migrate!().run(&pool).await?;
		Ok(Self { pool })
	}

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
