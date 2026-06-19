use super::Db;
use crate::error::{Result, db_insert_err, db_select_err};
use crate::user::User;
use sqlx::postgres::PgQueryResult;

impl Db {
	/// Selects a [User] from the database
	pub(crate) async fn get_user(&self, id: i32) -> Result<Option<User>> {
		sqlx::query_as!(
			User,
			"SELECT
				id, name, bio, link, followers,
				stories, blogs, profile_pic_url,
				color_hex, date_joined, date_cached
			FROM Authors WHERE id = $1 LIMIT 1;",
			id
		)
		.fetch_optional(&self.pool)
		.await
		.map_err(db_select_err)
	}

	/// Inserts a [User] into the database
	pub(crate) async fn insert_user(&self, user: &User) -> Result<PgQueryResult> {
		sqlx::query!(
			"INSERT INTO Authors
				(id, name, bio, link, followers, stories, blogs,
				profile_pic_url, color_hex, date_joined, date_cached)
			VALUES
				($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
			ON CONFLICT(id) DO UPDATE SET
				name = EXCLUDED.name,
				bio = EXCLUDED.bio,
				link = EXCLUDED.link,
				followers = EXCLUDED.followers,
				stories = EXCLUDED.stories,
				blogs = EXCLUDED.blogs,
				profile_pic_url = EXCLUDED.profile_pic_url,
				color_hex = EXCLUDED.color_hex,
				date_joined = EXCLUDED.date_joined,
				date_cached = EXCLUDED.date_cached;",
			user.id,
			user.name,
			user.bio,
			user.link,
			user.followers,
			user.stories,
			user.blogs,
			user.profile_pic_url,
			user.color_hex,
			user.date_joined,
			user.date_cached,
		)
		.execute(&self.pool)
		.await
		.map_err(db_insert_err)
	}
}
