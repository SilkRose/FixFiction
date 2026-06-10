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
mod story;
mod structs;
mod thread;
mod user;
mod utility;

use self::blog::{blog_html_template, request_blog};
use self::error::error_html_template;
use self::fimfiction_api::fimfic_api_headers;
use self::group::{group_html_template, request_group};
use self::oembed::get_oembed;
use self::structs::AppState;
use self::thread::{request_thread, thread_html_template};
use self::utility::{
	check_slash, check_thread_slash, parse_embed_parameters, parse_id, parse_thread_id,
};
use crate::bookshelf::get_bookshelf_endpoint;
use crate::chapter::get_chapter_endpoint;
use crate::story::get_story_endpoint;
use crate::user::get_user_endpoint;
use actix_cors::Cors;
use actix_web::middleware::Compress;
use actix_web::web::{Data, Path, Query};
use actix_web::{App, HttpResponse, HttpServer, Responder, get};
use pony::env::dotenv;
use pony::http::Request;
use reqwest::Client;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

/// The `blog/` endpoint.
///
/// Requests a blog by ID.
#[get("/blog/{id:.*}")]
async fn get_blog(
	path: Path<String>, queries: Query<HashMap<String, String>>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let blog_id = match parse_id(&path) {
		Ok(id) => id,
		Err(err) => {
			return Ok(HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(error_html_template("blog", path, err.to_string())));
		}
	};
	let (params, errors) = parse_embed_parameters(&mut path, queries, &app.db).await;
	let link = format!("https://www.fimfiction.net/blog/{path}");
	let body = match request_blog(blog_id, &app, params.refresh).await {
		Ok((blog, user, story)) => blog_html_template(blog, user, story, params, link, errors),
		Err(err) => error_html_template("blog", path, err.to_string()),
	};
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

/// The `group/` endpoint.
///
/// Requests a group by ID.
#[get("/group/{id:.*}")]
async fn get_group(
	path: Path<String>, queries: Query<HashMap<String, String>>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let group_id = match parse_id(&path) {
		Ok(id) => id,
		Err(err) => {
			return Ok(HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(error_html_template("group", path, err.to_string())));
		}
	};
	let thread_id = parse_thread_id(&path);
	if let Some(thread_id) = thread_id {
		check_thread_slash(&mut path, thread_id);
	} else {
		check_slash(&mut path, group_id);
	}
	let (params, errors) = parse_embed_parameters(&mut path, queries, &app.db).await;
	let link = format!("https://www.fimfiction.net/group/{path}");

	let body = match thread_id {
		Some(thread_id) => match request_thread(group_id, thread_id, &app, params.refresh).await {
			Ok((group, founder, thread_data)) => match thread_data {
				Some(thread_data) => {
					thread_html_template(group, founder, thread_data, params, link, errors)
				}
				None => group_html_template(group, founder, params, link, errors),
			},
			Err(err) => error_html_template("group", path, err.to_string()),
		},
		None => match request_group(group_id, &app, params.refresh).await {
			Ok((group, founder)) => group_html_template(group, founder, params, link, errors),
			Err(err) => error_html_template("group", path, err.to_string()),
		},
	};

	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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

	let app_data = AppState {
		api,
		db: db_pool,
		gc_interval: 3600,
		cache_max_age: 86400,
		cache_recache_age: 60,
	};

	HttpServer::new(move || {
		App::new()
			.app_data(Data::new(Arc::new(app_data.clone())))
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
			.service(get_blog)
			.service(get_group)
			.service(get_bookshelf_endpoint)
			.service(get_oembed)
	})
	.bind(("0.0.0.0", 7669))? // pony
	.run()
	.await?;

	Ok(())
}
