//! FixFiction is a service that fixes embedded content from Fimfiction.net.
mod blog;
mod bookshelf;
mod chapter;
mod database;
mod error;
mod fimfiction_api;
mod group;
mod html_template;
mod oembed;
mod parameters;
mod story;
mod tag;
mod thread;
mod user;
mod utility;

use self::fimfiction_api::fimfic_api_headers;
use self::oembed::get_oembed;
use crate::blog::get_blog_endpoint;
use crate::bookshelf::get_bookshelf_endpoint;
use crate::chapter::get_chapter_endpoint;
use crate::error::Result;
use crate::group::get_group_endpoint;
use crate::story::get_story_endpoint;
use crate::user::get_user_endpoint;
use actix_cors::Cors;
use actix_web::middleware::Compress;
use actix_web::web::ThinData;
use actix_web::{App, HttpServer};
use pony::env::dotenv;
use pony::http::Request;
use reqwest::Client;
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
	dotenv()?;

	// API Bearer token is required to scrape the data.
	let token = env::var("BEARER_TOKEN").expect("BEARER_TOKEN should be set");

	let user_agent = format!("FixFiction/{}", env!("CARGO_PKG_VERSION"));

	// API and site request structs, client, headers, and time intervals.
	let api = Request {
		client: Client::new(),
		headers: fimfic_api_headers(Some(&user_agent), &token)?,
		interval: Duration::from_millis(500),
		interval_step: Duration::from_secs(2),
		interval_max: Duration::from_secs(120),
		timeout: Duration::from_secs(10),
		max_tries: 4,
	};

	let database_url = env::var("DATABASE_URL").expect("DATABASE_URL should be set");
	let db_pool = sqlx::postgres::PgPoolOptions::new()
		.max_connections(16)
		.connect(&database_url)
		.await
		.expect("database should open");

	sqlx::migrate!("./migrations").run(&db_pool).await?;

	HttpServer::new(move || {
		App::new()
			.app_data(ThinData(api.clone()))
			.app_data(ThinData(db_pool.clone()))
			.wrap(
				Cors::default()
					.allow_any_origin()
					.allow_any_method()
					.allow_any_header()
					.max_age(3600),
			)
			.wrap(Compress::default())
			.service(get_story_endpoint)
			.service(get_chapter_endpoint)
			.service(get_user_endpoint)
			.service(get_blog_endpoint)
			.service(get_group_endpoint)
			.service(get_bookshelf_endpoint)
			.service(get_oembed)
	})
	.bind(("0.0.0.0", 7669))? // pony
	.run()
	.await?;

	Ok(())
}
