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
use crate::utility::LOG;
use actix_cors::Cors;
use actix_web::middleware::Compress;
use actix_web::web::ThinData;
use actix_web::{App, HttpServer};
use chrono::{Timelike, Utc};
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
	let client = Client::builder()
		.default_headers(api.headers)
		.build()
		.unwrap_or_else(|err| {
			eprintln!("Failed to create new client, using embed client! Error: {err:?}");
			api.client
		});
	loop {
		let time = Utc::now();
		let diff = 60_000 - (time.timestamp_millis() % 60_000) as u64;
		tokio::time::sleep(Duration::from_millis(diff)).await;
		let status = fimfic_status(&client, &db).await;
		if status == FimficStatus::Unreachable {
			continue;
		}
		if Utc::now().minute().is_multiple_of(10) {
			// fetch mew/updated/heat/featured
		}
		// archive code here
	}
}

async fn fimfic_status(api: &Client, db: &Db) -> FimficStatus {
	let status = fimfic_status_loop(api, db).await;
	if let Err(error) = db.insert_minute_status(&status).await {
		eprintln!("{error}");
	}
	match status.api_duration.is_some() {
		true => FimficStatus::Available,
		false => FimficStatus::Unreachable,
	}
}

async fn fimfic_status_loop(api: &Client, db: &Db) -> FimficStatusData {
	let mut history: Option<Vec<FimficStatusData>> = None;
	loop {
		let status = fimfic_status_request(api).await;
		if status.api_duration.is_some() {
			// Early return on success
			break status.flatten_seconds();
		} else {
			// Wait 5 seconds between attempts
			let time = status.datetime + Duration::from_secs(5);
			let time = (time - Utc::now()).num_milliseconds();
			if time > 0 {
				tokio::time::sleep(Duration::from_millis(time as u64)).await;
			}
		}
		if let Some(ref mut history) = history {
			if let Some(point) = history.pop()
				&& point.api_duration.is_some()
			{
				// Keep trying if the previous minute was successful
				continue;
			}
		} else {
			// Get latest history
			match db.get_last_n_status_minutes(5).await {
				Ok(latest) => {
					history = Some(latest);
					continue;
				}
				Err(err) => eprintln!("{err}"),
			};
		}
		// return if all tries fail
		break status.flatten_seconds();
	}
}

async fn fimfic_status_request(api: &Client) -> FimficStatusData {
	let url = "https://www.fimfiction.net/api/v2/bookshelves/1";
	let mut status = FimficStatusData::from(Utc::now());
	let res =
		tokio::time::timeout(Duration::from_secs(5), async { api.get(url).send().await }).await;
	let elapsed = (Utc::now() - status.datetime).num_milliseconds() as i32;
	match res {
		Ok(Ok(response)) => {
			status.round_trip = Some(elapsed);
			if let Some(header) = response.headers().get("cf-mitigated")
				&& header.as_bytes() == b"challenge"
			{
				if let Err(e) = LOG.warn("Fimfiction attack mode detected!") {
					eprintln!("{} - {e}", Utc::now());
				}
				status.challenged = true
			} else if let Ok(body) = response.bytes().await
				&& let Ok(bookshelf) = serde_json::from_slice::<BookshelfApi>(&body)
				&& let Ok(duration) = bookshelf.debug.duration.parse::<f64>()
			{
				status.api_duration = Some((duration * 1000.0) as i32);
			} else {
				if let Err(e) = LOG.warn("Fimfiction status request failed to parse") {
					eprintln!("{} - {e}", Utc::now());
				}
			}
		}
		Ok(Err(error)) => {
			let err = format!("Fimfiction status request failed: {error:?}");
			if let Err(e) = LOG.warn(&err) {
				eprintln!("{} - {e}", Utc::now());
			}
			status.round_trip = Some(elapsed);
		}
		Err(_) => {
			if let Err(e) = LOG.warn("Fimfiction status request timed out") {
				eprintln!("{} - {e}", Utc::now());
			}
		}
	}
	status
}
