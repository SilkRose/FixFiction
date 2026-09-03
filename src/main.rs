//! FixFiction is a service that fixes embedded content from Fimfiction.net.
mod blog;
mod bookshelf;
mod chapter;
mod database;
mod error;
mod fimfiction_api;
mod fimfiction_status;
mod group;
mod html_template;
mod oembed;
mod parameters;
mod story;
mod tag;
mod thread;
mod user;
mod utility;

use crate::blog::get_blog_endpoint;
use crate::bookshelf::get_bookshelf_endpoint;
use crate::chapter::get_chapter_endpoint;
use crate::database::Db;
use crate::error::Result;
use crate::fimfiction_api::bookshelf::BookshelfApi;
use crate::fimfiction_api::fimfic_api_headers;
use crate::fimfiction_status::{FimficStatus, FimficStatusData};
use crate::group::get_group_endpoint;
use crate::oembed::get_oembed;
use crate::story::{get_random_story_endpoint, get_story_endpoint};
use crate::user::get_user_endpoint;
use actix_cors::Cors;
use actix_web::middleware::Compress;
use actix_web::web::ThinData;
use actix_web::{App, HttpServer};
use chrono::Utc;
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
	let api_clone = api.clone();

	let database_url = env::var("DATABASE_URL").expect("DATABASE_URL should be set");
	let db_pool = Db::new(&database_url).await?;
	let db_clone = db_pool.clone();

	tokio::task::spawn(async move {
		let _ = archive_loop(api_clone.clone(), db_clone.clone()).await;
	});

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
			.service(get_random_story_endpoint)
			.service(get_oembed)
	})
	.bind(("0.0.0.0", 7669))? // pony
	.run()
	.await?;

	Ok(())
}

async fn archive_loop(api: Request, db: Db) {
	loop {
		let time = Utc::now();
		let diff = 60_000 - (time.timestamp_millis() % 60_000) as u64;
		tokio::time::sleep(Duration::from_millis(diff)).await;
		let status = fimfic_status(&api, &db).await;
		if status == FimficStatus::Unreachable {
			continue;
		}
		// archive code here
	}
}

async fn fimfic_status(api: &Request, db: &Db) -> FimficStatus {
	let url = "https://www.fimfiction.net/api/v2/bookshelves/1";
	let mut status = FimficStatusData::from(Utc::now());
	let res = tokio::time::timeout(Duration::from_secs(10), async {
		api.client
			.get(url)
			.headers(api.headers.clone())
			.send()
			.await
	})
	.await;
	let elapsed = (Utc::now() - status.datetime).num_milliseconds() as i32;
	match res {
		Ok(Ok(response)) => {
			status.round_trip = Some(elapsed);
			if let Ok(body) = response.bytes().await
				&& let Ok(bookshelf) = serde_json::from_slice::<BookshelfApi>(&body)
				&& let Ok(duration) = bookshelf.debug.duration.parse::<f64>()
			{
				status.api_duration = Some((duration * 1000.0) as i32);
			}
		}
		Ok(Err(error)) => {
			eprintln!("Fimfiction status request failed: {error}");
			status.round_trip = Some(elapsed);
		}
		Err(_) => {
			eprintln!("Fimfiction status request timed out");
		}
	}
	db.insert_status(&status).await.unwrap();
	match status.api_duration.is_some() {
		true => FimficStatus::Available,
		false => FimficStatus::Unreachable,
	}
}
