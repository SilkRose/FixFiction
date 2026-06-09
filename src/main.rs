//! FixFiction is a service that fixes embedded content from Fimfiction.net.
mod blog;
mod bookshelf;
mod chapter;
mod database;
mod error;
mod fimfiction_api;
mod group;
mod html_template;
mod story;
mod structs;
mod thread;
mod user;
mod utility;

use self::blog::{blog_html_template, request_blog};
use self::bookshelf::{bookshelf_html_template, request_bookshelf};
use self::chapter::{chapter_html_template, request_chapter, request_story_chapters};
use self::database::count_rows;
use self::error::error_html_template;
use self::fimfiction_api::fimfic_api_headers;
use self::group::{group_html_template, request_group};
use self::story::{request_story, story_html_template};
use self::structs::{AppState, OEmbed};
use self::thread::{request_thread, thread_html_template};
use self::user::{request_user, user_html_template};
use self::utility::{
	check_slash, check_thread_slash, parse_chapter_number, parse_embed_parameters, parse_id,
	parse_thread_id,
};
use actix_cors::Cors;
use actix_web::middleware::Compress;
use actix_web::web::{Data, Path, Query};
use actix_web::{App, HttpResponse, HttpServer, Responder, get};
use chrono::{TimeDelta, Utc};
use pony::env::dotenv;
use pony::http::Request;
use reqwest::Client;
use sqlx::{AssertSqlSafe, Executor};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

/// The `story/` endpoint.
///
/// Requests a story by ID.
/// May also include an ordinal chapter number.
#[get("/story/{id:.*}")]
async fn get_story(
	path: Path<String>, queries: Query<HashMap<String, String>>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let story_id = match parse_id(&path) {
		Ok(id) => id,
		Err(err) => {
			return Ok(HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(error_html_template("story", path, err.to_string())));
		}
	};
	let chapter_id = parse_chapter_number(&path);
	let (params, errors) = parse_embed_parameters(&mut path, queries, &app.db).await;
	let link = format!("https://www.fimfiction.net/story/{path}");
	let body = match chapter_id {
		Some(chapter_num) => {
			match request_story_chapters(story_id, chapter_num, &app, params.refresh).await {
				Ok((chapter, story, user, tags)) => {
					chapter_html_template(chapter, story, user, tags, params, link, errors)
				}
				Err(err) => error_html_template("story", path, err.to_string()),
			}
		}
		None => match request_story(story_id, &app, params.refresh).await {
			Ok((story, user, tags)) => story_html_template(story, user, tags, params, link, errors),
			Err(err) => error_html_template("story", path, err.to_string()),
		},
	};
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

/// The `chapter/` endpoint.
///
/// Requests a chapter by ID.
/// More direct than `story/{id}/chapter/{num}`.
#[get("/chapter/{id:.*}")]
async fn get_chapter(
	path: Path<String>, queries: Query<HashMap<String, String>>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let chapter_id = match parse_id(&path) {
		Ok(id) => id,
		Err(err) => {
			return Ok(HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(error_html_template("chapter", path, err.to_string())));
		}
	};
	let (params, errors) = parse_embed_parameters(&mut path, queries, &app.db).await;
	let link = format!("https://www.fimfiction.net/chapter/{path}");
	let body = match request_chapter(chapter_id, &app, params.refresh).await {
		Ok((chapter, story, user, tags)) => {
			chapter_html_template(chapter, story, user, tags, params, link, errors)
		}
		Err(err) => error_html_template("chapter", path, err.to_string()),
	};
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

/// The `user/` endpoint.
///
/// Requests a user by ID.
#[get("/user/{id:.*}")]
async fn get_user(
	path: Path<String>, queries: Query<HashMap<String, String>>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let user_id = match parse_id(&path) {
		Ok(id) => id,
		Err(err) => {
			return Ok(HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(error_html_template("user", path, err.to_string())));
		}
	};
	check_slash(&mut path, user_id);
	let (params, errors) = parse_embed_parameters(&mut path, queries, &app.db).await;
	let link = format!("https://www.fimfiction.net/user/{path}");
	let body = match request_user(user_id, &app, params.refresh).await {
		Ok(user) => user_html_template(user, params, link, errors),
		Err(err) => error_html_template("user", path, err.to_string()),
	};
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

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

/// The `bookshelf/` endpoint.
///
/// Requests a bookshelf by ID.
#[get("/bookshelf/{id:.*}")]
async fn get_bookshelf(
	path: Path<String>, queries: Query<HashMap<String, String>>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let bookshelf_id = match parse_id(&path) {
		Ok(id) => id,
		Err(err) => {
			return Ok(HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(error_html_template("bookshelf", path, err.to_string())));
		}
	};
	check_slash(&mut path, bookshelf_id);
	let (params, errors) = parse_embed_parameters(&mut path, queries, &app.db).await;
	let link = format!("https://www.fimfiction.net/bookshelf/{path}");
	let body = match request_bookshelf(bookshelf_id, &app, params.refresh).await {
		Ok((group, founder)) => bookshelf_html_template(group, founder, params, link, errors),
		Err(err) => error_html_template("bookshelf", path, err.to_string()),
	};
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

#[get("/oembed")]
async fn oembed(query: Query<OEmbed>) -> Result<impl Responder, Box<dyn Error>> {
	let embed = query.into_inner();
	Ok(HttpResponse::Ok()
		.content_type("application/json+oembed")
		.json(embed))
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

	let db_clone = db_pool.clone();
	let app_data = AppState {
		api,
		db: db_pool,
		gc_interval: 3600,
		cache_max_age: 86400,
		cache_recache_age: 60,
	};

	// Set up a task loop to decache old data occasionally
	tokio::task::spawn(async move {
		loop {
			let time = Utc::now() - TimeDelta::seconds(app_data.cache_max_age);
			let tables = [
				"Stories",
				"Chapters",
				"Tags",
				"Tag_links",
				"Authors",
				"Blogs",
				"Bookshelves",
				"Groups",
				"Threads",
			];
			for table in tables {
				let query = format!("DELETE FROM {table} WHERE date_cached < $1");
				if let Err(e) = db_clone
					.execute(sqlx::query(AssertSqlSafe(query)).bind(time))
					.await
				{
					eprintln!("Failed to delete from {table}: {e}");
				}
			}
			let stories = count_rows("Stories", &db_clone).await.unwrap();
			let chapters = count_rows("Chapters", &db_clone).await.unwrap();
			let tags = count_rows("Tags", &db_clone).await.unwrap();
			let tag_links = count_rows("Tag_links", &db_clone).await.unwrap();
			let users = count_rows("Authors", &db_clone).await.unwrap();
			let blogs = count_rows("Blogs", &db_clone).await.unwrap();
			let bookshelves = count_rows("Bookshelves", &db_clone).await.unwrap();
			let groups = count_rows("Groups", &db_clone).await.unwrap();
			let threads = count_rows("Threads", &db_clone).await.unwrap();
			let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
			print!("{time}: stories: {stories}, chapters: {chapters},");
			print!(" tags: {tags}, tag links: {tag_links}");
			print!(" users: {users}, blogs: {blogs}, bookshelves: {bookshelves},");
			println!(" groups: {groups}, threads: {threads}");
			tokio::time::sleep(Duration::from_secs(app_data.gc_interval)).await;
		}
	});

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
			.service(get_story)
			.service(get_chapter)
			.service(get_user)
			.service(get_blog)
			.service(get_group)
			.service(get_bookshelf)
			.service(oembed)
	})
	.bind(("0.0.0.0", 7669))? // pony
	.run()
	.await?;

	Ok(())
}
