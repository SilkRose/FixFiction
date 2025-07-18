use actix_cors::Cors;
use actix_web::web::{Data, Path, Query};
use actix_web::{App, HttpResponse, HttpServer, Responder, get};
use chrono::{TimeDelta, Utc};
use dotenvy::dotenv;
use fixfiction::blog::{blog_html_template, request_blog};
use fixfiction::bookshelf::{bookshelf_html_template, request_bookshelf};
use fixfiction::chapter::{chapter_html_template, request_chapter, request_story_chapters};
use fixfiction::database::count_rows;
use fixfiction::error::error_html_template;
use fixfiction::fimfiction_api::fimfic_api_headers;
use fixfiction::group::{group_html_template, request_group};
use fixfiction::prune_db;
use fixfiction::story::{request_story, story_html_template};
use fixfiction::structs::{AppState, OEmbed};
use fixfiction::user::{request_user, user_html_template};
use fixfiction::utility::{check_slash, parse_embed_parameters, parse_id, parse_second_id};
use pony::http::Request;
use reqwest::Client;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

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
	let chapter_id = parse_second_id(&path);
	let (params, errors) = parse_embed_parameters(&mut path, queries, &app.db).await;
	let link = format!("https://www.fimfiction.net/story/{path}");
	let body = match chapter_id {
		Some(chapter_num) => {
			match request_story_chapters(story_id, chapter_num, &app, params.refresh).await {
				Ok((chapter, story, user)) => {
					chapter_html_template(chapter, story, user, params, link, errors)
				}
				Err(err) => error_html_template("story", path, err.to_string()),
			}
		}
		None => match request_story(story_id, &app, params.refresh).await {
			Ok((story, user)) => story_html_template(story, user, params, link, errors),
			Err(err) => error_html_template("story", path, err.to_string()),
		},
	};
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

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
		Ok((chapter, story, user)) => {
			chapter_html_template(chapter, story, user, params, link, errors)
		}
		Err(err) => error_html_template("chapter", path, err.to_string()),
	};
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

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
	check_slash(&mut path, group_id);
	let (params, errors) = parse_embed_parameters(&mut path, queries, &app.db).await;
	let link = format!("https://www.fimfiction.net/group/{path}");
	let body = match request_group(group_id, &app, params.refresh).await {
		Ok((group, founder)) => group_html_template(group, founder, params, link, errors),
		Err(err) => error_html_template("group", path, err.to_string()),
	};
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

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

	// API and site request structs, client, headers, and time intervals.
	let api = Request {
		client: Client::new(),
		headers: fimfic_api_headers(Some("FixFiction"), &token)?,
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

	tokio::task::spawn(async move {
		loop {
			let time = Utc::now() - TimeDelta::seconds(app_data.cache_max_age);
			prune_db!(
				"DELETE FROM Bookshelves WHERE date_cached < $1",
				time,
				db_clone
			);
			prune_db!("DELETE FROM Blogs WHERE date_cached < $1", time, db_clone);
			prune_db!("DELETE FROM Authors WHERE date_cached < $1", time, db_clone);
			prune_db!("DELETE FROM Stories WHERE date_cached < $1", time, db_clone);
			prune_db!("DELETE FROM Groups WHERE date_cached < $1", time, db_clone);
			let bookshelves = count_rows("Bookshelves", &db_clone).await.unwrap();
			let blogs = count_rows("Blogs", &db_clone).await.unwrap();
			let users = count_rows("Authors", &db_clone).await.unwrap();
			let stories = count_rows("Stories", &db_clone).await.unwrap();
			let chapters = count_rows("Chapters", &db_clone).await.unwrap();
			let groups = count_rows("Groups", &db_clone).await.unwrap();
			let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
			println!(
				"{time} -- stories: {stories}, users: {users}, blogs: {blogs}, chapters: {chapters}, groups: {groups}, bookshelves: {bookshelves}"
			);
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
