use actix_cors::Cors;
use actix_web::web::{Data, Path, Query};
use actix_web::{App, HttpResponse, HttpServer, Responder, get};
use chrono::{DateTime, TimeDelta, Utc};
use core::str;
use dotenvy::dotenv;
use pony::fimfiction_api::blog::BlogApi;
use pony::fimfiction_api::fimfic_api_headers;
use pony::fimfiction_api::story::StoryApi;
use pony::fimfiction_api::user::{UserApi, UserData};
use pony::http::{Request, api_get_request};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Type, query};
use std::env;
use std::error::Error;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
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
	date_published: DateTime<Utc>,
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
	date_joined: DateTime<Utc>,
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
	date_posted: DateTime<Utc>,
	date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
struct Parameters {
	#[serde(alias = "image")]
	cover: Option<Cover>,
	color: Option<Color>,
	#[serde(default)]
	refresh: bool,
	#[serde(default)]
	stats: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
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
	Story,
	User,
	None,
}

impl From<String> for Color {
	fn from(value: String) -> Self {
		match value.as_str() {
			"story" => Color::Story,
			"user" => Color::User,
			"none" => Color::None,
			_ => Color::Custom(value),
		}
	}
}

#[derive(Debug, Clone)]
struct AppState {
	api: Request,
	db: Pool<Postgres>,
	gc_interval: u64,
	cache_max_age: i64,
	cache_recache_age: i64,
}

#[get("/story/{id:.*}")]
async fn get_story(
	path: Path<String>, query: Query<Parameters>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let path = path.into_inner();
	let mut params = query.into_inner();
	let id = parse_parameters(&path, &mut params, &app.db).await?;
	let link = format!("https://www.fimfiction.net/story/{path}");
	let (story, user) = request_story(id, &app, params.refresh).await?;
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(story_html_template(story, user, params, link)))
}

#[get("/user/{id:.*}")]
async fn get_user(
	path: Path<String>, query: Query<Parameters>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let path = path.into_inner();
	let mut params = query.into_inner();
	let id = parse_parameters(&path, &mut params, &app.db).await?;
	let link = format!("https://www.fimfiction.net/user/{path}");
	let user = request_user(id, &app, params.refresh).await?;
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(user_html_template(user, params, link)))
}

#[get("/blog/{id:.*}")]
async fn get_blog(
	path: Path<String>, query: Query<Parameters>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let path = path.into_inner();
	let mut params = query.into_inner();
	let id = parse_parameters(&path, &mut params, &app.db).await?;
	let link = format!("https://www.fimfiction.net/blog/{path}");
	let (blog, user, story) = request_blog(id, &app, params.refresh).await?;
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
		} else if color.len() == 6 {
			params.color = color
				.as_bytes()
				.iter()
				.all(|hex| hex.is_ascii_hexdigit())
				.then_some(Color::Custom(color.to_string()));
		} else {
			params.color = None;
		}
	}
	Ok(id)
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

async fn request_story(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Story, User), Box<dyn std::error::Error>> {
	let story = sqlx::query_as!(
		Story,
		r#"SELECT
			id, title, short_description, cover_medium_url,
			color_hex, views, words, chapters, comments,
			completion_status AS "completion_status: CompletionStatus",
			content_rating AS "content_rating: ContentRating",
			likes, dislikes, author_id, date_published, date_cached
		FROM Stories WHERE id = $1 LIMIT 1;"#,
		id
	)
	.fetch_optional(&app.db)
	.await?;

	let story = match recache {
		true => story.filter(|story| {
			Utc::now()
				.checked_sub_signed(TimeDelta::seconds(app.cache_recache_age))
				.is_some_and(|max_age| story.date_cached >= max_age)
		}),
		false => story,
	};

	match story {
		Some(story) => {
			let user = request_user(story.author_id, app, recache).await?;
			Ok((story, user))
		}
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/stories/{id}");
			let response = api_get_request(&app.api, &fimfic).await.unwrap();
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
					likes, dislikes, author_id, date_published)
				VALUES
					($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
				ON CONFLICT(id) DO UPDATE SET
					title = EXCLUDED.title,
					short_description = EXCLUDED.short_description,
					cover_medium_url = EXCLUDED.cover_medium_url,
					color_hex = EXCLUDED.color_hex,
					views = EXCLUDED.views,
					words = EXCLUDED.words,
					chapters = EXCLUDED.chapters,
					comments = EXCLUDED.comments,
					completion_status = EXCLUDED.completion_status,
					content_rating = EXCLUDED.content_rating,
					likes = EXCLUDED.likes,
					dislikes = EXCLUDED.dislikes,
					author_id = EXCLUDED.author_id,
					date_published = EXCLUDED.date_published,
					date_cached = now()
				RETURNING 
					id, title, short_description, cover_medium_url,
					color_hex, views, words, chapters, comments,
					completion_status AS "completion_status: CompletionStatus",
					content_rating AS "content_rating: ContentRating",
					likes, dislikes, author_id, date_published, date_cached;"#,
				id,
				clean_content(api.data.attributes.title),
				clean_content(api.data.attributes.short_description),
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
				user.id,
				DateTime::parse_from_rfc3339(
					&api.data
						.attributes
						.date_published
						.expect("All published stories should be published.")
				)?
			)
			.fetch_one(&app.db)
			.await?;
			Ok((story, user))
		}
	}
}

async fn request_user(
	id: i32, app: &AppState, recache: bool,
) -> Result<User, Box<dyn std::error::Error>> {
	let user = sqlx::query_as!(
		User,
		"SELECT
			id, name, bio, link, followers,
			stories, blogs, profile_pic_256,
			color_hex, date_joined, date_cached
		FROM Authors WHERE id = $1 LIMIT 1;",
		id
	)
	.fetch_optional(&app.db)
	.await?;

	let user = match recache {
		true => user.filter(|user| {
			Utc::now()
				.checked_sub_signed(TimeDelta::seconds(app.cache_recache_age))
				.is_some_and(|max_age| user.date_cached >= max_age)
		}),
		false => user,
	};

	match user {
		Some(user) => Ok(user),
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/users/{id}");
			let response = api_get_request(&app.api, &fimfic).await.unwrap();
			let api = response.json::<UserApi<i32>>().await.unwrap();
			response_to_user(&api.data, &app.db).await
		}
	}
}

async fn request_blog(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Blog, User, Option<Story>), Box<dyn std::error::Error>> {
	let blog = sqlx::query_as!(
		Blog,
		"SELECT
			id, title, content, comments, views,
			author_id, story_id, date_posted, date_cached
		FROM Blogs WHERE id = $1 LIMIT 1;",
		id
	)
	.fetch_optional(&app.db)
	.await?;

	let blog = match recache {
		true => blog.filter(|blog| {
			Utc::now()
				.checked_sub_signed(TimeDelta::seconds(app.cache_recache_age))
				.is_some_and(|max_age| blog.date_cached >= max_age)
		}),
		false => blog,
	};

	match blog {
		Some(blog) => {
			let (story, user) = if let Some(story_id) = blog.story_id {
				let (story, user) = request_story(story_id, app, recache).await?;
				(Some(story), user)
			} else {
				(None, request_user(blog.author_id, app, recache).await?)
			};
			Ok((blog, user, story))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/blog-posts/{id}?include=author&fields[blog_post]=title,date_posted,content,num_views,num_comments,tagged_story"
			);
			let response = api_get_request(&app.api, &fimfic).await.unwrap();
			let api = response.json::<BlogApi<i32>>().await?;
			let author = api.included.first().unwrap();
			let story_id = (api.data.relationships.tagged_story.data.id != "0")
				.then_some(api.data.relationships.tagged_story.data.id.parse::<i32>()?);
			let (story, user) = if let Some(story_id) = story_id {
				let (story, user) = request_story(story_id, app, recache).await?;
				(Some(story), user)
			} else {
				(None, response_to_user(author, &app.db).await?)
			};
			let blog = sqlx::query_as!(
				Blog,
				"INSERT INTO Blogs 
					(id, title, content, comments, views,
					author_id, story_id, date_posted)
				VALUES
					($1, $2, $3, $4, $5, $6, $7, $8)
				ON CONFLICT(id) DO UPDATE SET
					title = EXCLUDED.title,
					content = EXCLUDED.content,
					comments = EXCLUDED.comments,
					views = EXCLUDED.views,
					author_id = EXCLUDED.author_id,
					story_id = EXCLUDED.story_id,
					date_posted = EXCLUDED.date_posted,
					date_cached = now()
				RETURNING
					id, title, content, comments, views,
					author_id, story_id, date_posted,
					date_cached;",
				api.data.id.parse::<i32>()?,
				clean_content(api.data.attributes.title),
				trim_content(api.data.attributes.content, true),
				api.data.attributes.num_comments,
				api.data.attributes.num_views,
				author.id.parse::<i32>()?,
				story_id,
				DateTime::parse_from_rfc3339(&api.data.attributes.date_posted)?
			)
			.fetch_one(&app.db)
			.await?;
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
			blogs, profile_pic_256, color_hex, date_joined)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
		ON CONFLICT(id) DO UPDATE SET
			name = EXCLUDED.name,
			bio = EXCLUDED.bio,
			link = EXCLUDED.link,
			followers = EXCLUDED.followers,
			stories = EXCLUDED.stories,
			blogs = EXCLUDED.blogs,
			profile_pic_256 = EXCLUDED.profile_pic_256,
			color_hex = EXCLUDED.color_hex,
			date_joined = EXCLUDED.date_joined,
			date_cached = now()
		RETURNING
			id, name, bio, link, followers,
			stories, blogs, profile_pic_256,
			color_hex, date_joined, date_cached;",
		data.id.parse::<i32>()?,
		clean_content(data.attributes.name.clone()),
		clean_content(data.attributes.bio.clone()),
		data.meta.url,
		data.attributes.num_followers,
		data.attributes.num_stories,
		data.attributes.num_blog_posts,
		image,
		data.attributes.color.hex.trim_start_matches("#"),
		DateTime::parse_from_rfc3339(&data.attributes.date_joined)?
	)
	.fetch_one(db)
	.await?;
	Ok(user)
}

fn story_html_template(story: Story, user: User, parameters: Parameters, link: String) -> String {
	let mut text = String::new();
	text.push_str(r#"<!DOCTYPE html><html lang="en"><head>"#);
	text.push_str("<!-- FixFiction: https://github.com/SilkRose/FixFiction -->");
	text.push_str("<!-- Pinkie Pie is best pony! -->");
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::User => Some(user.color_hex),
			Color::Story => Some(story.color_hex),
		},
		None => Some(story.color_hex),
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
	text.push_str(&format!(
		r#"<meta property="og:title" content="{}" />"#,
		story.title
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{}" />"#,
		story.short_description
	));
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::Story => story.cover_medium_url,
			Cover::User => user.profile_pic_256,
			Cover::None => None,
		},
		None => story.cover_medium_url,
	};
	if let Some(cover) = cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
	text.push_str(&format!(r#"<meta property="og:url" content="{link}" />"#));
	text.push_str(r#"<meta property="og:type" content="book" />"#);
	text.push_str(&format!(
		r#"<meta property="book:author" content="{}" />"#,
		user.link
	));
	let site_name = if parameters.stats {
		let time = story.date_published.format("%a %b %e %Y").to_string();
		let status = match story.completion_status {
			CompletionStatus::Incomplete => "Incomplete 🔄",
			CompletionStatus::Complete => "Complete ✅",
			CompletionStatus::Hiatus => "Hiatus ⏸",
			CompletionStatus::Cancelled => "Cancelled ❌",
		};
		let rating = match story.content_rating {
			ContentRating::Everyone => "Everyone 🇪",
			ContentRating::Teen => "Teen 🇹",
			ContentRating::Mature => "Mature 🇲",
		};
		let likes_dislikes = if story.likes == -1 && story.dislikes == -1 {
			String::new()
		} else {
			format!("Likes: {} 👍 Dislikes: {} 👎 ", story.likes, story.dislikes)
		};
		&format!(
			"Fimfiction - Published: {time} 📅 Status: {status}\nRating: {rating} {likes_dislikes}Views: {} 📈\nComments: {} 💬 Chapters: {} 📖 Words: {} 📝",
			story.views, story.comments, story.chapters, story.words
		)
	} else {
		"Fimfiction"
	};
	text.push_str(&format!(
		r#"<meta property="og:site_name" content="{site_name}" />"#
	));
	text.push_str(r#"<meta property="twitter:site" content="fimfiction" />"#);
	text.push_str(r#"<meta property="twitter:card" content="summary" />"#);
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair("type", "rich");
	encode.append_pair("version", "1");
	encode.append_pair("provider_name", site_name);
	encode.append_pair("provider_url", "https://www.fimfiction.net/");
	encode.append_pair("title", &story.title);
	encode.append_pair("author_name", &user.name);
	encode.append_pair("author_url", &user.link);
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	let encode = encode.finish();
	text.push_str(&format!(
		r#"<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed?{encode}" title="{}" />"#,
		user.name));
	text.push_str(r#"</head><body></body></html>"#);
	text
}

fn user_html_template(user: User, parameters: Parameters, link: String) -> String {
	let mut text = String::new();
	text.push_str(r#"<!DOCTYPE html><html lang="en"><head>"#);
	text.push_str("<!-- FixFiction: https://github.com/SilkRose/FixFiction -->");
	text.push_str("<!-- Pinkie Pie is best pony! -->");
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			_ => Some(user.color_hex),
		},
		None => Some(user.color_hex),
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
	text.push_str(&format!(
		r#"<meta property="og:title" content="{}" />"#,
		user.name
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{}" />"#,
		user.bio
	));
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::None => None,
			Cover::User => user.profile_pic_256,
			Cover::Story => user.profile_pic_256,
		},
		None => user.profile_pic_256,
	};
	if let Some(cover) = cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
	text.push_str(&format!(r#"<meta property="og:url" content="{link}" />"#));
	text.push_str(r#"<meta property="og:type" content="profile" />"#);
	text.push_str(&format!(
		r#"<meta property="profile:username" content="{}" />"#,
		user.name
	));
	let site_name = if parameters.stats {
		{
			let time = user.date_joined.format("%a %b %e %Y").to_string();
			&format!(
				"Fimfiction - Joined: {time} 📅\nStories: {} 📚 Blogs: {} 📑 Followers: {} 👥",
				user.stories, user.blogs, user.followers
			)
		}
	} else {
		"Fimfiction"
	};
	text.push_str(&format!(
		r#"<meta property="og:site_name" content="{site_name}" />"#
	));
	text.push_str(r#"<meta property="twitter:site" content="fimfiction" />"#);
	text.push_str(r#"<meta property="twitter:card" content="summary" />"#);
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair("type", "rich");
	encode.append_pair("version", "1");
	encode.append_pair("provider_name", site_name);
	encode.append_pair("provider_url", "https://www.fimfiction.net/");
	encode.append_pair("title", &user.name);
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	let encode = encode.finish();
	text.push_str(&format!(
			r#"<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed?{encode}" title="{}" />"#,
		user.name));
	text.push_str(r#"</head><body></body></html>"#);
	text
}

fn blog_html_template(
	blog: Blog, user: User, story: Option<Story>, parameters: Parameters, link: String,
) -> String {
	let mut text = String::new();
	text.push_str(r#"<!DOCTYPE html><html lang="en"><head>"#);
	text.push_str("<!-- FixFiction: https://github.com/SilkRose/FixFiction -->");
	text.push_str("<!-- Pinkie Pie is best pony! -->");
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::Story => Some(
				story
					.clone()
					.map(|story| story.color_hex)
					.unwrap_or(user.color_hex),
			),
			Color::User => Some(user.color_hex),
		},
		None => Some(user.color_hex),
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
	text.push_str(&format!(
		r#"<meta property="og:title" content="{}" />"#,
		blog.title
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{}" />"#,
		blog.content
	));
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::User => user.profile_pic_256,
			Cover::Story => story
				.map(|story| story.cover_medium_url)
				.unwrap_or(user.profile_pic_256),
			Cover::None => None,
		},
		None => user.profile_pic_256,
	};
	if let Some(cover) = cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
	text.push_str(&format!(r#"<meta property="og:url" content="{link}" />"#));
	text.push_str(r#"<meta property="og:type" content="article" />"#);
	text.push_str(&format!(
		r#"<meta property="article:author" content="{}" />"#,
		user.link
	));
	text.push_str(&format!(
		r#"<meta property="article:published_time" content="{}" />"#,
		blog.date_posted
	));
	let site_name = if parameters.stats {
		let time = blog.date_posted.format("%a %b %e %Y").to_string();
		&format!(
			"Fimfiction - Posted: {time} 📅\nViews: {} 📈 Comments: {} 💬",
			blog.views, blog.comments
		)
	} else {
		"Fimfiction"
	};
	text.push_str(&format!(
		r#"<meta property="og:site_name" content="{site_name}" />"#
	));
	text.push_str(r#"<meta property="twitter:site" content="fimfiction" />"#);
	text.push_str(r#"<meta property="twitter:card" content="summary" />"#);
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair("type", "rich");
	encode.append_pair("version", "1");
	encode.append_pair("provider_name", site_name);
	encode.append_pair("provider_url", "https://www.fimfiction.net/");
	encode.append_pair("title", &blog.title);
	encode.append_pair("author_name", &user.name);
	encode.append_pair("author_url", &user.link);
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	let encode = encode.finish();
	text.push_str(&format!(
		r#"<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed?{encode}" title="{}" />"#,
		user.name));
	text.push_str(r#"</head><body></body></html>"#);
	text
}
