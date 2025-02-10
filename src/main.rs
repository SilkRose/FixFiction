use actix_cors::Cors;
use actix_web::web::{Data, Path};
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use chrono::{DateTime, Local, Utc};
use dotenvy::dotenv;
use pony::fimfiction_api::blog::BlogApi;
use pony::fimfiction_api::story::StoryApi;
use pony::fimfiction_api::user::{UserApi, UserData};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Type};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "content_rating", rename_all = "lowercase")]
enum ContentRating {
	Everyone,
	Teen,
	Mature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "completion_status", rename_all = "lowercase")]
enum CompletionStatus {
	Incomplete,
	Complete,
	Hiatus,
	Cancelled,
}

#[derive(Debug, Clone)]
struct FimficRequest {
	client: Client,
	headers: HeaderMap,
	interval: Duration,
	interval_step: Duration,
	interval_max: Duration,
	timeout: Duration,
}

#[derive(Debug, Clone)]
struct Story {
	id: u32,
	link: String,
	title: String,
	color: String,
	author: Author,
	o_embed: OEmbed,
	timestamp: u128,
	short_description: String,
	cover_medium_url: Option<String>,
}

#[derive(Debug, Clone)]
struct StoryDB {
	id: i32,
	title: String,
	short_description: String,
	cover_medium_url: Option<String>,
	color_hex: String,
	views: i32,
	words: i32,
	chapters: i32,
	comments: i32,
	completion_status: CompletionStatus,
	content_rating: ContentRating,
	likes: i32,
	dislikes: i32,
	author_id: i32,
	date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct Author {
	id: u32,
	url: String,
	name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct OEmbed {
	r#type: String,
	version: u32,
	provider_name: String,
	provider_url: String,
	title: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	author_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	author_url: Option<String>,
	cache_age: u32,
	html: String,
}

#[derive(Debug, Clone)]
struct User {
	id: u32,
	link: String,
	name: String,
	color: String,
	o_embed: OEmbed,
	timestamp: u128,
	bio_bbcode: String,
	profile_pic_256_url: Option<String>,
}

#[derive(Debug, Clone)]
struct UserDB {
	id: i32,
	name: String,
	bio: String,
	link: String,
	followers: i32,
	stories: i32,
	blogs: i32,
	profile_pic_256: Option<String>,
	color_hex: String,
	date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct Blog {
	id: u32,
	link: String,
	title: String,
	author_id: u32,
	o_embed: OEmbed,
	timestamp: u128,
	_story_id: Option<u32>,
	content_bbcode: String,
	date_published: String,
}

#[derive(Debug, Clone)]
struct BlogDB {
	id: i32,
	title: String,
	content: String,
	comments: i32,
	views: i32,
	author_id: i32,
	story_id: Option<i32>,
	date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct AppState {
	api: FimficRequest,
	stories: Arc<RwLock<HashMap<u32, Story>>>,
	stories_meta: MetaData,
	users: Arc<RwLock<HashMap<u32, User>>>,
	users_meta: MetaData,
	blogs: Arc<RwLock<HashMap<u32, Blog>>>,
	blogs_meta: MetaData,
}

#[derive(Clone, Debug, Default)]
struct MetaData {
	fixfic_requests: Arc<AtomicU32>,
	fimfic_requests: Arc<AtomicU32>,
	rate_limit_limit: Arc<AtomicU32>,
	rate_limit_remaining: Arc<AtomicU32>,
	rate_limit_reset: Arc<AtomicU32>,
}

macro_rules! garbage_collector {
	($fun:ident, $T:ty, $name:literal) => {
		fn $fun(
			data: Arc<RwLock<HashMap<u32, $T>>>, meta: &MetaData, time: u128,
		) -> Result<String, Box<dyn std::error::Error>> {
			let mut data = data.write().expect("Failed to lock data");
			let total = data.len();
			data.retain(|_, story| story.timestamp > time);
			let requests = &meta.fixfic_requests.load(Ordering::Acquire);
			meta.fixfic_requests.store(0, Ordering::Release);
			let remaining = data.len();
			let dropped = total - remaining;
			let text = format!("{}: ({requests}, {dropped}, {remaining})", $name);
			Ok(text)
		}
	};
}

garbage_collector!(story_cleanup, Story, "story");
garbage_collector!(user_cleanup, User, "user");
garbage_collector!(blog_cleanup, Blog, "blog");

#[get("/story/{id:.*}")]
async fn get_story(
	path: web::Path<String>, data: web::Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
	let ident = path.into_inner();
	let link = format!("https://www.fimfiction.net/story/{ident}");
	let ident = ident.split('/').collect::<Vec<_>>();
	let ident = ident.first().unwrap();
	let ident = ident.parse::<u32>().unwrap();
	data.stories_meta
		.fixfic_requests
		.fetch_add(1, Ordering::AcqRel);
	let local_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
	let stories = data.stories.read().map_err(|_| "Failed to lock data")?;
	if let Some(story) = stories.get(&ident) {
		println!("{local_time}: [story] cache hit:  {ident}");
		Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(story_template(story, link)))
	} else {
		drop(stories);
		println!("{local_time}: [story] cache miss: {ident}");
		let (story, user) = request_story(ident, &data.api, &data.stories_meta).await?;
		let mut users = data.users.write().map_err(|_| "Failed to lock data")?;
		users.insert(story.author.id, user.clone());
		let mut stories = data.stories.write().map_err(|_| "Failed to lock data")?;
		stories.insert(ident, story.clone());
		Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(story_template(&story, link)))
	}
}

macro_rules! o_embed {
	($endpoint:literal, $fun:ident, $items:ident) => {
		#[get($endpoint)]
		async fn $fun(
			path: web::Path<String>, data: web::Data<Arc<AppState>>,
		) -> Result<impl Responder, Box<dyn std::error::Error>> {
			let ident = path.into_inner().parse::<u32>().unwrap();
			let items = data.$items.read().map_err(|_| "Failed to lock data")?;
			if let Some(item) = items.get(&ident) {
				Ok(HttpResponse::Ok()
					.content_type("application/json+oembed")
					.json(item.o_embed.clone()))
			} else {
				Ok(HttpResponse::NotFound().finish())
			}
		}
	};
}

o_embed!("/oembed/story/{id}", oembed_story, stories);
o_embed!("/oembed/user/{id}", oembed_user, users);
o_embed!("/oembed/blog/{id}", oembed_blog, blogs);

#[get("/user/{id:.*}")]
async fn get_user(
	path: Path<String>, data: Data<Arc<AppState>>, db: Data<Pool<Postgres>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let ident = path.into_inner();
	let link = format!("https://www.fimfiction.net/user/{ident}");
	let ident = ident.split('/').collect::<Vec<_>>();
	let ident = ident.first().unwrap();
	let ident = ident.parse::<i32>().unwrap();
	let user = request_user_db(ident, &data.api, db).await?;
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(format!("{user:?}")))
}

#[get("/blog/{id:.*}")]
async fn get_blog(
	path: web::Path<String>, data: web::Data<Arc<AppState>>, db: Data<Pool<Postgres>>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
	let ident = path.into_inner();
	let link = format!("https://www.fimfiction.net/blog/{ident}");
	let ident = ident.split('/').collect::<Vec<_>>();
	let ident = ident.first().unwrap();
	let ident = ident.parse::<i32>().unwrap();
	let blog = request_blog_db(ident, &data.api, db).await?;
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(format!("{blog:?}")))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	dotenv()?;

	// API Bearer token is required to scrape the data.
	let token = &env::args().collect::<Vec<_>>()[1];

	// API and site request structs, client, headers, and time intervals.
	let api = FimficRequest {
		client: Client::new(),
		headers: setup_api_headers(token)?,
		interval: Duration::from_millis(500),
		interval_step: Duration::from_secs(2),
		interval_max: Duration::from_secs(120),
		timeout: Duration::from_secs(10),
	};

	let app_data = AppState {
		api: api.clone(),
		stories: Arc::new(RwLock::new(HashMap::<u32, Story>::new())),
		stories_meta: MetaData::default(),
		users: Arc::new(RwLock::new(HashMap::<u32, User>::new())),
		users_meta: MetaData::default(),
		blogs: Arc::new(RwLock::new(HashMap::<u32, Blog>::new())),
		blogs_meta: MetaData::default(),
	};

	let state_clone = app_data.clone();

	let database_url = env::var("DATABASE_URL").expect("DATABASE_URL should be set");
	let db_pool = sqlx::postgres::PgPoolOptions::new()
		.max_connections(16)
		.connect(&database_url)
		.await
		.expect("database should open");

	sqlx::migrate!("./migrations").run(&db_pool).await?;

	// Seconds between garbage collection.
	const GC: u64 = 3600;
	// Milliseconds to keep a story cached.
	const TTL: u128 = 86_400_000;

	tokio::task::spawn(async move {
		loop {
			tokio::time::sleep(Duration::from_secs(GC)).await;
			let time = unix_time().unwrap() - TTL;
			let local_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
			let stories =
				story_cleanup(state_clone.stories.clone(), &state_clone.stories_meta, time)
					.unwrap();
			let users =
				user_cleanup(state_clone.users.clone(), &state_clone.users_meta, time).unwrap();
			let blogs =
				blog_cleanup(state_clone.blogs.clone(), &state_clone.blogs_meta, time).unwrap();
			println!(
				"{local_time} <- (requests, dropped, remaining) -> {stories}, {users}, {blogs}"
			)
		}
	});

	HttpServer::new(move || {
		App::new()
			.app_data(Data::new(db_pool.clone()))
			.app_data(Data::new(Arc::new(app_data.clone())))
			.wrap(
				Cors::default()
					.allow_any_origin()
					.allow_any_method()
					.allow_any_header()
					.max_age(3600),
			)
			.service(get_story)
			.service(oembed_story)
			.service(get_user)
			.service(oembed_user)
			.service(get_blog)
			.service(oembed_blog)
	})
	.bind(("0.0.0.0", 7669))? // pony
	.run()
	.await?;

	Ok(())
}

fn setup_api_headers(token: &str) -> Result<HeaderMap, Box<dyn Error>> {
	let mut headers = HeaderMap::new();
	headers.insert(
		AUTHORIZATION,
		HeaderValue::from_str(&format!("Bearer {}", token))?,
	);
	headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
	Ok(headers)
}

async fn handle_request(
	request: &FimficRequest, url: &str,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
	let mut interval = request.interval;
	loop {
		let start_time = unix_time()?;
		let res = timeout(
			request.timeout,
			request
				.client
				.get(url)
				.headers(request.headers.clone())
				.send(),
		)
		.await;
		match res {
			Ok(Ok(response)) => {
				let limit = response.headers().get("x-rate-limit-limit");
				let remaining = response.headers().get("x-rate-limit-remaining");
				let reset = response.headers().get("x-rate-limit-reset");
				println!("limit: {limit:?}, remaining: {remaining:?}, reset: {reset:?}");
				return Ok(response);
			}
			Ok(Err(error)) => {
				println!("Request failed: {error}");
			}
			Err(error) => {
				println!("Request timed out: {error}");
			}
		}
		sleep(start_time, interval).await?;
		interval = if interval < request.interval_max {
			interval + request.interval_step
		} else {
			request.interval_max
		};
	}
}

async fn sleep(
	start_time: u128, interval: Duration,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let current_time = unix_time()?;
	let elapsed_time = Duration::from_millis((current_time - start_time).try_into()?);
	if elapsed_time > interval {
		return Ok(());
	};
	tokio::time::sleep(interval - elapsed_time).await;
	Ok(())
}

fn unix_time() -> Result<u128, Box<dyn std::error::Error + Send + Sync>> {
	Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
}

async fn request_story(
	id: u32, api: &FimficRequest, meta: &MetaData,
) -> Result<(Story, User), Box<dyn Error>> {
	let fimfic = format!("https://www.fimfiction.net/api/v2/stories/{id}");
	let response = handle_request(api, &fimfic).await.unwrap();
	let api = response.json::<StoryApi>().await.unwrap();
	let author = api
		.included
		.iter()
		.find(|author| author.id == api.data.relationships.author.data.id)
		.unwrap();
	let user = response_to_user(author.clone())?;
	let author = Author {
		id: author.id.parse()?,
		url: user.clone().link,
		name: user.clone().name,
	};
	let story = api.data;
	let title = story.attributes.title.replace('"', "&quot;");
	let story = Story {
		id: story.id.parse::<u32>().unwrap(),
		link: story.meta.url,
		title: title.clone(),
		color: story.attributes.color.hex,
		author: author.clone(),
		o_embed: create_o_embed(title, Some(author), false),
		timestamp: unix_time().unwrap(),
		short_description: story.attributes.short_description.replace('"', "&quot;"),
		cover_medium_url: story.attributes.cover_image.map(|cover| cover.medium),
	};
	Ok((story, user))
}

async fn request_story_db(
	id: i32, api: &FimficRequest, db: Data<Pool<Postgres>>,
) -> Result<StoryDB, Box<dyn std::error::Error>> {
	let story = sqlx::query_as!(
		StoryDB,
		r#"SELECT
		id, title, short_description, cover_medium_url, color_hex, views, words, chapters, comments,
		completion_status AS "completion_status: CompletionStatus", content_rating AS "content_rating: ContentRating",
		likes, dislikes, author_id, date_cached
		FROM Stories WHERE id = $1 LIMIT 1"#,
		id
	)
	.fetch_optional(&**db)
	.await?;
	match story {
		Some(story) => {
			let user = request_user_db(story.author_id, api, db).await?;
			Ok(story)
		}
		None => {
			// Need to finish story DB code.
			todo!()
		}
	}
}

async fn request_user_db(
	id: i32, api: &FimficRequest, db: Data<Pool<Postgres>>,
) -> Result<UserDB, Box<dyn std::error::Error>> {
	let user = sqlx::query_as!(UserDB, "SELECT * FROM Authors WHERE id = $1 LIMIT 1", id)
		.fetch_optional(&**db)
		.await?;
	match user {
		Some(user) => Ok(user),
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/users/{id}");
			let response = handle_request(api, &fimfic).await.unwrap();
			let api = response.json::<UserApi<i32>>().await.unwrap();
			response_to_user_db(&api.data, db).await
		}
	}
}

async fn request_blog_db(
	id: i32, api: &FimficRequest, db: Data<Pool<Postgres>>,
) -> Result<BlogDB, Box<dyn std::error::Error>> {
	let blog = sqlx::query_as!(BlogDB, "SELECT * FROM Blogs WHERE id = $1 LIMIT 1", id)
		.fetch_optional(&**db)
		.await?;
	match blog {
		Some(blog) => {
			let user = request_user_db(blog.author_id, api, db).await?;
			Ok(blog)
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/blog-posts/{id}?include=author&fields[blog_post]=title,date_posted,content,num_views,num_comments,tagged_story"
			);
			let response = handle_request(api, &fimfic).await.unwrap();
			let api = response.json::<BlogApi<i32>>().await?;
			let author = api.included.first().unwrap();
			let user = response_to_user_db(author, db.clone()).await?;
			let story_id = (api.data.relationships.tagged_story.data.id != "0")
				.then_some(api.data.relationships.tagged_story.data.id.parse::<i32>()?);
			let blog = sqlx::query_as!(
				BlogDB,
				"INSERT INTO Blogs 
					(id, title, content, comments, views, author_id, story_id)
				VALUES
					($1, $2, $3, $4, $5, $6, $7)
				ON CONFLICT(id) DO UPDATE SET
					title = EXCLUDED.title,
					content = EXCLUDED.content,
					comments = EXCLUDED.comments,
					views = EXCLUDED.views,
					author_id = EXCLUDED.author_id,
					story_id = EXCLUDED.story_id,
					date_cached = now()
				RETURNING *;",
				api.data.id.parse::<i32>()?,
				clean_content(api.data.attributes.title),
				trim_content(api.data.attributes.content, true),
				api.data.attributes.num_comments,
				api.data.attributes.num_views,
				user.id,
				story_id,
			)
			.fetch_one(&**db)
			.await?;
			Ok(blog)
		}
	}
}

fn trim_content(content: String, clean: bool) -> String {
	let mut text = vec![];
	let mut chars = 0;
	for line in content.lines() {
		if chars + line.len() < 512 {
			text.push(line);
			chars += line.len() + 1;
		} else {
			break;
		}
	}
	match clean {
		true => clean_content(text.join("\n")),
		false => text.join("\n"),
	}
}

fn clean_content(content: String) -> String {
	let re = LazyLock::new(|| Regex::new(r"\[[^]]+\]").unwrap());
	re.replace_all(&content, "")
		.to_string()
		.replace('"', "&quot;")
}

fn response_to_user(data: UserData) -> Result<User, Box<dyn std::error::Error>> {
	let image = (!data.attributes.avatar.r64.ends_with("none_64.png"))
		.then_some(data.attributes.avatar.r256);
	let re = LazyLock::new(|| Regex::new(r"\[[^]]+\]").unwrap());
	let name = data.attributes.name.replace('"', "&quot;");
	let user = User {
		id: data.id.parse()?,
		link: data.meta.url,
		name: name.clone(),
		color: data.attributes.color.hex,
		o_embed: create_o_embed(name, None, false),
		timestamp: unix_time().unwrap(),
		bio_bbcode: re
			.replace_all(&data.attributes.bio, "")
			.to_string()
			.replace('"', "&quot;"),
		profile_pic_256_url: image,
	};
	Ok(user)
}

async fn response_to_user_db(
	data: &UserData<i32>, db: Data<Pool<Postgres>>,
) -> Result<UserDB, Box<dyn Error>> {
	let image = (!data.attributes.avatar.r64.ends_with("none_64.png"))
		.then_some(data.attributes.avatar.r256.clone());
	let user = sqlx::query_as!(
		UserDB,
		"INSERT INTO Authors 
			(id, name, bio, link, followers, stories, blogs, profile_pic_256, color_hex)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9)
		ON CONFLICT(id) DO UPDATE SET
			name = EXCLUDED.name,
			bio = EXCLUDED.bio,
			link = EXCLUDED.link,
			followers = EXCLUDED.followers,
			stories = EXCLUDED.stories,
			blogs = EXCLUDED.blogs,
			profile_pic_256 = EXCLUDED.profile_pic_256,
			color_hex = EXCLUDED.color_hex,
			date_cached = now()
		RETURNING *;",
		data.id.parse::<i32>()?,
		clean_content(data.attributes.name.clone()),
		clean_content(data.attributes.bio.clone()),
		data.meta.url,
		data.attributes.num_followers,
		data.attributes.num_stories,
		data.attributes.num_blog_posts,
		image,
		data.attributes.color.hex.trim_start_matches("#")
	)
	.fetch_one(&**db)
	.await?;
	Ok(user)
}

fn create_o_embed(title: String, author: Option<Author>, error: bool) -> OEmbed {
	let (name, url) = match error {
		false => (
			String::from("Fimfiction"),
			String::from("https://www.fimfiction.net/"),
		),
		true => (
			String::from("Fixfiction"),
			String::from("https://www.fixfiction.net/"),
		),
	};
	OEmbed {
		r#type: String::from("rich"),
		version: 1,
		provider_name: name,
		provider_url: url,
		title: title.replace('"', "&quot;"),
		author_name: author.clone().map(|author| author.name),
		author_url: author.map(|author| author.url),
		cache_age: 86_400,
		html: String::default(),
	}
}

fn story_template(story: &Story, link: String) -> String {
	match &story.cover_medium_url {
		Some(cover) => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{story_link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{title}" />
		<meta property="og:description" content="{}" />
		<meta property="og:image" content="{}" />
		<meta property="og:url" content="{story_link}" />
		<meta property="og:type" content="book" />
		<meta property="book:author" content="{}" />
		<meta property="og:site_name" content="Fimfiction" />
		<meta property="twitter:site" content="fimfiction" />
		<meta property="twitter:card" content="summary" />
		<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed/story/{}" title="{}" />
	</head>
	<body></body>
	</html>"#,
			story.color,
			story.short_description,
			cover,
			story.author.url,
			story.id,
			story.author.name,
			title = story.title,
			story_link = story.link,
		),
		None => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{story_link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{title}" />
		<meta property="og:description" content="{}" />
		<meta property="og:url" content="{story_link}" />
		<meta property="og:type" content="book" />
		<meta property="book:author" content="{}" />
		<meta property="og:site_name" content="Fimfiction" />
		<meta property="twitter:site" content="fimfiction" />
		<meta property="twitter:card" content="summary" />
		<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed/story/{}" title="{}" />
	</head>
	<body></body>
	</html>"#,
			story.color,
			story.short_description,
			story.author.url,
			story.id,
			story.author.name,
			title = story.title,
			story_link = story.link,
		),
	}
}

fn user_template(user: &User, link: String) -> String {
	match &user.profile_pic_256_url {
		Some(image) => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{user_link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{name}" />
		<meta property="og:description" content="{}" />
		<meta property="og:image" content="{}" />
		<meta property="og:url" content="{user_link}" />
		<meta property="og:type" content="profile" />
		<meta property="profile:username" content="{name}" />
		<meta property="og:site_name" content="Fimfiction" />
		<meta property="twitter:site" content="fimfiction" />
		<meta property="twitter:card" content="summary" />
		<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed/user/{}" title="{name}" />
	</head>
	<body></body>
	</html>"#,
			user.color,
			user.bio_bbcode,
			image,
			user.id,
			name = user.name,
			user_link = user.link
		),
		None => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{user_link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{name}" />
		<meta property="og:description" content="{}" />
		<meta property="og:url" content="{user_link}" />
		<meta property="og:type" content="profile" />
		<meta property="profile:username" content="{name}" />
		<meta property="og:site_name" content="Fimfiction" />
		<meta property="twitter:site" content="fimfiction" />
		<meta property="twitter:card" content="summary" />
		<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed/user/{}" title="{name}" />
	</head>
	<body></body>
	</html>"#,
			user.color,
			user.bio_bbcode,
			user.id,
			name = user.name,
			user_link = user.link
		),
	}
}

fn blog_template(blog: &Blog, user: &User, link: String) -> String {
	match &user.profile_pic_256_url {
		Some(image) => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{blog_link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{title}" />
		<meta property="og:description" content="{}" />
		<meta property="og:image" content="{}" />
		<meta property="og:url" content="{blog_link}" />
		<meta property="og:type" content="article" />
		<meta property="article:author" content="{}" />
		<meta property="article:published_time" content="{}" />
		<meta property="og:site_name" content="Fimfiction" />
		<meta property="twitter:site" content="fimfiction" />
		<meta property="twitter:card" content="summary" />
		<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed/blog/{}" title="{title}" />
	</head>
	<body></body>
	</html>"#,
			user.color,
			blog.content_bbcode,
			image,
			user.link,
			blog.date_published,
			blog.id,
			blog_link = blog.link,
			title = blog.title,
		),
		None => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{blog_link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{title}" />
		<meta property="og:description" content="{}" />
		<meta property="og:url" content="{blog_link}" />
		<meta property="og:type" content="article" />
		<meta property="article:author" content="{}" />
		<meta property="article:published_time" content="{}" />
		<meta property="og:site_name" content="Fimfiction" />
		<meta property="twitter:site" content="fimfiction" />
		<meta property="twitter:card" content="summary" />
		<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed/blog/{}" title="{title}" />
	</head>
	<body></body>
	</html>"#,
			user.color,
			blog.content_bbcode,
			user.link,
			blog.date_published,
			blog.id,
			blog_link = blog.link,
			title = blog.title,
		),
	}
}
