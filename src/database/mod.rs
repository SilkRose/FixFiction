//! Reading and writing changes to the database.

use crate::bookshelf::Bookshelf;
use crate::chapter::Chapter;
use crate::error::{Result, db_delete_err, db_insert_err, db_select_err};
use crate::group::Group;
use crate::tag::{Tag, TagLink, TagType};
use crate::thread::Thread;
use sqlx::postgres::{PgPoolOptions, PgQueryResult};
use sqlx::{Pool, Postgres};

mod blog;
mod story;
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
