//! Reading and writing changes to the database.

use crate::error::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

mod blog;
mod bookshelf;
mod chapter;
mod group;
mod story;
mod tag;
mod tag_link;
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
}
