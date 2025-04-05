use actix_cors::Cors;
use actix_web::web::{Data, Path, Query};
use actix_web::{App, HttpResponse, HttpServer, Responder, get};
use chrono::{TimeDelta, Utc};
use dotenvy::dotenv;
use fixfiction::blog::{blog_html_template, request_blog};
use fixfiction::story::{request_story, story_html_template};
use fixfiction::structs::{AppState, OEmbed, Parameters};
use fixfiction::user::{request_user, user_html_template};
use fixfiction::utility::{parse_id, parse_parameters, parse_second_id};
use pony::fimfiction_api::fimfic_api_headers;
use pony::http::Request;
use reqwest::Client;
use sqlx::query;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

#[get("/story/{id:.*}")]
async fn get_story(
	path: Path<String>, query: Query<Parameters>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let path = path.into_inner();
	let story_id = parse_id(&path)?;
	let mut params = query.into_inner();
	let chapter_id = parse_second_id(&path);
	parse_parameters(&mut params, &app.db).await?;
	let link = format!("https://www.fimfiction.net/story/{path}");
	let (story, user) = request_story(story_id, &app, params.refresh).await?;
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(story_html_template(story, user, params, link)))
}

#[get("/user/{id:.*}")]
async fn get_user(
	path: Path<String>, query: Query<Parameters>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let path = path.into_inner();
	let user_id = parse_id(&path)?;
	let mut params = query.into_inner();
	parse_parameters(&mut params, &app.db).await?;
	let link = format!("https://www.fimfiction.net/user/{path}");
	let user = request_user(user_id, &app, params.refresh).await?;
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(user_html_template(user, params, link)))
}

#[get("/blog/{id:.*}")]
async fn get_blog(
	path: Path<String>, query: Query<Parameters>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let path = path.into_inner();
	let blog_id = parse_id(&path)?;
	let mut params = query.into_inner();
	parse_parameters(&mut params, &app.db).await?;
	let link = format!("https://www.fimfiction.net/blog/{path}");
	let (blog, user, story) = request_blog(blog_id, &app, params.refresh).await?;
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(blog_html_template(blog, user, story, params, link)))
}

#[get("/oembed")]
async fn oembed(query: Query<OEmbed>) -> Result<impl Responder, Box<dyn Error>> {
	let embed = query.into_inner();
	Ok(HttpResponse::Ok()
		.content_type("application/json+oembed")
		.json(embed))
}

macro_rules! prune_db {
	($query:literal, $time:ident, $db:ident) => {
		query!($query, $time).execute(&$db).await.unwrap()
	};
}

macro_rules! count_rows {
	($query:literal, $db:ident) => {
		query!($query).fetch_one(&$db).await.unwrap().count.unwrap()
	};
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
			tokio::time::sleep(Duration::from_secs(app_data.gc_interval)).await;
			let time = Utc::now() - TimeDelta::seconds(app_data.cache_max_age);
			prune_db!("DELETE FROM Blogs WHERE date_cached < $1", time, db_clone);
			prune_db!("DELETE FROM Authors WHERE date_cached < $1", time, db_clone);
			prune_db!("DELETE FROM Stories WHERE date_cached < $1", time, db_clone);
			let blogs = count_rows!("SELECT count(*) FROM Blogs;", db_clone);
			let users = count_rows!("SELECT count(*) FROM Authors;", db_clone);
			let stories = count_rows!("SELECT count(*) FROM Stories;", db_clone);
			let time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
			println!("{time}: stories: {stories}, users: {users}, blogs: {blogs}");
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
			.service(get_user)
			.service(get_blog)
			.service(oembed)
	})
	.bind(("0.0.0.0", 7669))? // pony
	.run()
	.await?;

	Ok(())
}
