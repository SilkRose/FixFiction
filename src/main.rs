use actix_cors::Cors;
use actix_web::web::{Data, Path, Query};
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use chrono::{DateTime, TimeDelta, Utc};
use core::str;
use dotenvy::dotenv;
use pony::fimfiction_api::blog::BlogApi;
use pony::fimfiction_api::story::StoryApi;
use pony::fimfiction_api::user::{UserApi, UserData};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use sqlx::{query, Pool, Postgres, Type};
use std::env;
use std::error::Error;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;
use url::form_urlencoded;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "content_rating", rename_all = "lowercase")]
enum ContentRating {
	Everyone,
	Teen,
	Mature,
}

impl From<String> for ContentRating {
	fn from(value: String) -> Self {
		match value.as_str() {
			"everyone" => ContentRating::Everyone,
			"teen" => ContentRating::Teen,
			"mature" => ContentRating::Mature,
			_ => unreachable!(), // This should never happen, but still want to add something here later.
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "completion_status", rename_all = "lowercase")]
enum CompletionStatus {
	Incomplete,
	Complete,
	Hiatus,
	Cancelled,
}

impl From<String> for CompletionStatus {
	fn from(value: String) -> Self {
		match value.as_str() {
			"incomplete" => CompletionStatus::Incomplete,
			"complete" => CompletionStatus::Complete,
			"hiatus" => CompletionStatus::Hiatus,
			"cancelled" => CompletionStatus::Cancelled,
			_ => unreachable!(), // This should never happen, but still want to add something here later.
		}
	}
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
	id: i32,
	title: String,
	content: String,
	comments: i32,
	views: i32,
	author_id: i32,
	story_id: Option<i32>,
	date_published: String,
	date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone)]
enum TemplateType {
	Story(Story, User),
	User(User),
	Blog(Blog, User, Option<Story>),
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
struct Parameters {
	cover: Option<Cover>,
	color: Option<Color>,
	#[serde(default)]
	refresh: bool,
	#[serde(default)]
	stats: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase", try_from = "String")]
enum Cover {
	Story,
	User,
	None,
}

impl TryFrom<String> for Cover {
	type Error = &'static str;
	fn try_from(value: String) -> Result<Self, Self::Error> {
		match value.as_str() {
			"story" => Ok(Cover::Story),
			"user" => Ok(Cover::User),
			"none" => Ok(Cover::None),
			_ => Err("invalid cover value"),
		}
	}
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase", try_from = "String")]
enum Color {
	Custom(String),
	Default,
	Story,
	User,
	None,
}

impl From<String> for Color {
	fn from(value: String) -> Self {
		match value.as_str() {
			"default" => Color::Default,
			"story" => Color::Story,
			"user" => Color::User,
			"none" => Color::None,
			_ => Color::Custom(value),
		}
	}
}

#[derive(Debug, Clone)]
struct AppState {
	api: FimficRequest,
	db: Pool<Postgres>,
}

#[get("/story/{id:.*}")]
async fn get_story(
	path: Path<String>, query: Query<Parameters>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let path = path.into_inner();
	let mut params = query.into_inner();
	let id = parse_parameters(&path, &mut params, &app.db).await?;
	let link = format!("https://www.fimfiction.net/story/{path}");
	let (story, user) = request_story(id, &app).await?;
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(html_template(
			TemplateType::Story(story, user),
			params,
			link,
		)))
}

#[get("/user/{id:.*}")]
async fn get_user(
	path: Path<String>, query: Query<Parameters>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let path = path.into_inner();
	let mut params = query.into_inner();
	let id = parse_parameters(&path, &mut params, &app.db).await?;
	let link = format!("https://www.fimfiction.net/user/{path}");
	let user = request_user(id, &app).await?;
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(html_template(TemplateType::User(user), params, link)))
}

#[get("/blog/{id:.*}")]
async fn get_blog(
	path: Path<String>, query: Query<Parameters>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let path = path.into_inner();
	let mut params = query.into_inner();
	let id = parse_parameters(&path, &mut params, &app.db).await?;
	let link = format!("https://www.fimfiction.net/blog/{path}");
	let (blog, user, story) = request_blog(id, &app).await?;
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(html_template(
			TemplateType::Blog(blog, user, story),
			params,
			link,
		)))
}

#[get("/oembed")]
async fn oembed(query: Query<OEmbed>) -> Result<impl Responder, Box<dyn Error>> {
	let embed = query.into_inner();
	Ok(HttpResponse::Ok()
		.content_type("application/json+oembed")
		.json(embed))
}

async fn parse_parameters(
	path: &str, params: &mut Parameters, db: &Pool<Postgres>,
) -> Result<i32, Box<dyn Error>> {
	let binding = path.to_string();
	let id = binding.split('/').collect::<Vec<_>>();
	let id = id.first().unwrap();
	let id = id.parse::<i32>().unwrap();
	if let Some(Color::Custom(color)) = &params.color {
		let db_color = query!("SELECT color FROM Colors WHERE name = $1 LIMIT 1;", color)
			.fetch_optional(db)
			.await?;
		if let Some(color) = db_color {
			params.color = Some(Color::Custom(color.color));
		} else if color.len() == 6 && color.is_ascii() {
			params.color = color
				.as_bytes()
				.chunks(2)
				.all(|hex| u8::from_str_radix(unsafe { str::from_utf8_unchecked(hex) }, 16).is_ok())
				.then_some(Color::Custom(color.to_string()));
		} else {
			params.color = None;
		}
	}
	Ok(id)
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

	let database_url = env::var("DATABASE_URL").expect("DATABASE_URL should be set");
	let db_pool = sqlx::postgres::PgPoolOptions::new()
		.max_connections(16)
		.connect(&database_url)
		.await
		.expect("database should open");

	sqlx::migrate!("./migrations").run(&db_pool).await?;

	let db_clone = db_pool.clone();

	const GC: u64 = 3600;
	const TTL: i64 = 86400;

	tokio::task::spawn(async move {
		loop {
			tokio::time::sleep(Duration::from_secs(GC)).await;
			let time = Utc::now().checked_sub_signed(TimeDelta::seconds(TTL));
			query!(
				"CALL garbage_collector(ARRAY['Blogs', 'Authors', 'Stories'], $1);",
				time.unwrap()
			)
			.execute(&db_clone)
			.await
			.unwrap();
			let counts = query!("SELECT count_rows(ARRAY['Blogs', 'Authors', 'Stories']);")
				.fetch_one(&db_clone)
				.await
				.unwrap()
				.count_rows
				.unwrap();
			println!(
				"blogs: {}, users: {}, stories: {}",
				counts[0], counts[1], counts[2]
			)
		}
	});

	let app_data = AppState { api, db: db_pool };

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
	id: i32, app: &AppState,
) -> Result<(Story, User), Box<dyn std::error::Error>> {
	let story = sqlx::query_as!(
		Story,
		r#"SELECT
			id, title, short_description, cover_medium_url,
			color_hex, views, words, chapters, comments,
			completion_status AS "completion_status: CompletionStatus",
			content_rating AS "content_rating: ContentRating",
			likes, dislikes, author_id, date_cached
		FROM Stories WHERE id = $1 LIMIT 1;"#,
		id
	)
	.fetch_optional(&app.db)
	.await?;
	match story {
		Some(story) => {
			let user = request_user(story.author_id, app).await?;
			Ok((story, user))
		}
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/stories/{id}");
			let response = handle_request(&app.api, &fimfic).await.unwrap();
			let api = response.json::<StoryApi<i32>>().await.unwrap();
			let author = api
				.included
				.iter()
				.find(|author| author.id == api.data.relationships.author.data.id)
				.unwrap();
			let user = response_to_user(&author.clone(), &app.db).await?;
			let story = sqlx::query_as!(
				Story,
				r#"INSERT INTO Stories (
					id, title, short_description, cover_medium_url,
					color_hex, views, words, chapters, comments,
					completion_status, content_rating,
					likes, dislikes, author_id)
				VALUES
					($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
				RETURNING 
					id, title, short_description, cover_medium_url,
					color_hex, views, words, chapters, comments,
					completion_status AS "completion_status: CompletionStatus",
					content_rating AS "content_rating: ContentRating",
					likes, dislikes, author_id, date_cached;"#,
				id,
				api.data.attributes.title,
				api.data.attributes.short_description,
				api.data.attributes.cover_image.map(|cover| cover.medium),
				api.data.attributes.color.hex.trim_start_matches("#"),
				api.data.attributes.num_views,
				api.data.attributes.num_words,
				api.data.attributes.num_chapters,
				api.data.attributes.num_comments,
				CompletionStatus::from(api.data.attributes.completion_status) as _,
				ContentRating::from(api.data.attributes.content_rating) as _,
				api.data.attributes.num_likes,
				api.data.attributes.num_dislikes,
				user.id
			)
			.fetch_one(&app.db)
			.await?;
			Ok((story, user))
		}
	}
}

async fn request_user(id: i32, app: &AppState) -> Result<User, Box<dyn std::error::Error>> {
	let user = sqlx::query_as!(
		User,
		"SELECT
			id, name, bio, link, followers,
			stories, blogs, profile_pic_256,
			color_hex, date_cached
		FROM Authors WHERE id = $1 LIMIT 1;",
		id
	)
	.fetch_optional(&app.db)
	.await?;
	match user {
		Some(user) => Ok(user),
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/users/{id}");
			let response = handle_request(&app.api, &fimfic).await.unwrap();
			let api = response.json::<UserApi<i32>>().await.unwrap();
			response_to_user(&api.data, &app.db).await
		}
	}
}

async fn request_blog(
	id: i32, app: &AppState,
) -> Result<(Blog, User, Option<Story>), Box<dyn std::error::Error>> {
	let blog = sqlx::query_as!(
		Blog,
		"SELECT
			id, title, content, comments, views,
			author_id, story_id, date_published,
			date_cached
		FROM Blogs WHERE id = $1 LIMIT 1;",
		id
	)
	.fetch_optional(&app.db)
	.await?;
	match blog {
		Some(blog) => {
			let user = request_user(blog.author_id, app).await?;
			Ok((blog, user, None))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/blog-posts/{id}?include=author&fields[blog_post]=title,date_posted,content,num_views,num_comments,tagged_story"
			);
			let response = handle_request(&app.api, &fimfic).await.unwrap();
			let api = response.json::<BlogApi<i32>>().await?;
			let author = api.included.first().unwrap();
			let user = response_to_user(author, &app.db).await?;
			let story_id = (api.data.relationships.tagged_story.data.id != "0")
				.then_some(api.data.relationships.tagged_story.data.id.parse::<i32>()?);
			let blog = sqlx::query_as!(
				Blog,
				"INSERT INTO Blogs 
					(id, title, content, comments, views,
					author_id, story_id, date_published)
				VALUES
					($1, $2, $3, $4, $5, $6, $7, $8)
				ON CONFLICT(id) DO UPDATE SET
					title = EXCLUDED.title,
					content = EXCLUDED.content,
					comments = EXCLUDED.comments,
					views = EXCLUDED.views,
					author_id = EXCLUDED.author_id,
					story_id = EXCLUDED.story_id,
					date_published = EXCLUDED.date_published,
					date_cached = now()
				RETURNING
					id, title, content, comments, views,
					author_id, story_id, date_published,
					date_cached;",
				api.data.id.parse::<i32>()?,
				clean_content(api.data.attributes.title),
				trim_content(api.data.attributes.content, true),
				api.data.attributes.num_comments,
				api.data.attributes.num_views,
				user.id,
				story_id,
				api.data.attributes.date_posted
			)
			.fetch_one(&app.db)
			.await?;
			let story = if let Some(story_id) = blog.story_id {
				let (story, _) = request_story(story_id, app).await?;
				Some(story)
			} else {
				None
			};
			Ok((blog, user, story))
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

async fn response_to_user(
	data: &UserData<i32>, db: &Pool<Postgres>,
) -> Result<User, Box<dyn Error>> {
	let image = (!data.attributes.avatar.r64.ends_with("none_64.png"))
		.then_some(data.attributes.avatar.r256.clone());
	let user = sqlx::query_as!(
		User,
		"INSERT INTO Authors 
			(id, name, bio, link, followers, stories,
			blogs, profile_pic_256, color_hex)
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
		RETURNING
			id, name, bio, link, followers,
			stories, blogs, profile_pic_256,
			color_hex, date_cached;",
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
	.fetch_one(db)
	.await?;
	Ok(user)
}

fn html_template(data: TemplateType, parameters: Parameters, link: String) -> String {
	let mut text = String::new();
	text.push_str(r#"<!DOCTYPE html><html lang="en"><head>"#);
	let color = match parameters.color {
		Some(color) => match (data.clone(), color) {
			(TemplateType::Story(story, _), Color::Default) => Some(story.color_hex),
			(TemplateType::Story(story, _), Color::Story) => Some(story.color_hex),
			(TemplateType::Story(_, user), Color::User) => Some(user.color_hex),
			(TemplateType::Blog(_, user, story), Color::Story) => {
				Some(story.map(|story| story.color_hex).unwrap_or(user.color_hex))
			}
			(TemplateType::Blog(_, user, _), _) => Some(user.color_hex),
			(TemplateType::User(user), _) => Some(user.color_hex),
			(_, Color::Custom(color)) => Some(color),
			(_, Color::None) => None,
		},
		None => match data.clone() {
			TemplateType::Story(story, _) => Some(story.color_hex),
			TemplateType::User(user) => Some(user.color_hex),
			TemplateType::Blog(_, user, _) => Some(user.color_hex),
		},
	};
	if let Some(color) = color {
		text.push_str(&format!(
			r##"<meta name="theme-color" content="#{color}" />"##
		));
	}
	text.push_str(&format!(r#"<link rel="canonical" href="{link}" />"#));
	text.push_str(&format!(
		r#"<meta http-equiv="refresh" content="0;url={link}" />"#
	));
	let (title, description) = match data.clone() {
		TemplateType::Story(story, _) => (story.title, story.short_description),
		TemplateType::User(user) => (user.name, user.bio),
		TemplateType::Blog(blog, _, _) => (blog.title, blog.content),
	};
	text.push_str(&format!(
		r#"<meta property="og:title" content="{title}" />"#
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{description}" />"#
	));
	let cover = match parameters.cover {
		Some(cover) => match (data.clone(), cover) {
			(TemplateType::Story(story, _), Cover::Story) => story.cover_medium_url,
			(TemplateType::Story(_, user), Cover::User) => user.profile_pic_256,
			(TemplateType::User(user), Cover::Story | Cover::User) => user.profile_pic_256,
			(TemplateType::Blog(_, user, _), Cover::User) => user.profile_pic_256,
			(TemplateType::Blog(_, user, story), Cover::Story) => story
				.map(|story| story.cover_medium_url)
				.unwrap_or(user.profile_pic_256),
			(_, Cover::None) => None,
		},
		None => match data.clone() {
			TemplateType::Story(story, _) => story.cover_medium_url,
			TemplateType::User(user) => user.profile_pic_256,
			TemplateType::Blog(_, user, _) => user.profile_pic_256,
		},
	};
	if let Some(cover) = cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
	text.push_str(&format!(r#"<meta property="og:url" content="{link}" />"#));
	let (og_type, property, content) = match data.clone() {
		TemplateType::Story(_, user) => ("book", "book:author", user.link),
		TemplateType::User(user) => ("profile", "profile:username", user.name),
		TemplateType::Blog(_, user, _) => ("article", "article:author", user.link),
	};
	text.push_str(&format!(
		r#"<meta property="og:type" content="{og_type}" />"#
	));
	text.push_str(&format!(
		r#"<meta property="{property}" content="{content}" />"#
	));
	if let TemplateType::Blog(blog, _, _) = data.clone() {
		text.push_str(&format!(
			r#"<meta property="article:published_time" content="{}" />"#,
			blog.date_published
		));
	}
	text.push_str(r#"<meta property="og:site_name" content="Fimfiction" />"#);
	text.push_str(r#"<meta property="twitter:site" content="fimfiction" />"#);
	text.push_str(r#"<meta property="twitter:card" content="summary" />"#);
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair("type", "rich");
	encode.append_pair("version", "1");
	encode.append_pair("provider_name", "Fimfiction");
	encode.append_pair("provider_url", "https://www.fimfiction.net/");
	encode.append_pair("title", &title);
	match data {
		TemplateType::Story(_, user) => {
			encode.append_pair("author_name", &user.name);
			encode.append_pair("author_url", &user.link);
		}
		TemplateType::Blog(_, user, _) => {
			encode.append_pair("author_name", &user.name);
			encode.append_pair("author_url", &user.link);
		}
		_ => {}
	}
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	let encode = encode.finish();
	text.push_str(&format!(r#"<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed?{encode}" title="{title}" />"#));
	text.push_str(r#"</head><body></body></html>"#);
	text
}
