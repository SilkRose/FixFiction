use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use chrono::Local;
use pony::fimfiction_api::blog::BlogApi;
use pony::fimfiction_api::story::StoryApi;
use pony::fimfiction_api::user::UserApi;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;

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
	requests: u32,
	color: String,
	author: Author,
	o_embed: OEmbed,
	timestamp: u128,
	short_description: String,
	cover_medium_url: Option<String>,
}

#[derive(Debug, Clone)]
struct Author {
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
	author_name: Option<String>,
	author_url: Option<String>,
	cache_age: u32,
	html: String,
}

#[derive(Debug, Clone)]
struct User {
	link: String,
	name: String,
	requests: u32,
	color: String,
	o_embed: OEmbed,
	timestamp: u128,
	bio_bbcode: String,
	profile_pic_256_url: Option<String>,
}

#[derive(Debug, Clone)]
struct Blog {
	link: String,
	title: String,
	requests: u32,
	author_id: u32,
	o_embed: OEmbed,
	timestamp: u128,
	story_id: Option<u32>,
	content_bbcode: String,
	date_published: String,
}

#[derive(Debug, Clone)]
struct AppState {
	api: FimficRequest,
	stories: Arc<RwLock<HashMap<u32, Story>>>,
	users: Arc<RwLock<HashMap<u32, User>>>,
	blogs: Arc<RwLock<HashMap<u32, Blog>>>,
}

macro_rules! garbage_collector {
	($fun:ident, $T:ty, $name:literal) => {
		fn $fun(
			data: Arc<RwLock<HashMap<u32, $T>>>, time: u128,
		) -> Result<String, Box<dyn std::error::Error>> {
			let mut data = data.write().expect("Failed to lock data");
			let total = data.len();
			let requests = data.iter().map(|(_, item)| item.requests).sum::<u32>();
			data.retain(|_, story| story.timestamp > time);
			data.iter_mut().for_each(|(_, item)| item.requests = 0);
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
	let ident = ident.split('/').collect::<Vec<_>>();
	let ident = ident.first().unwrap();
	let ident = ident.parse::<u32>().unwrap();
	let mut stories = data.stories.write().map_err(|_| "Failed to lock data")?;
	if let Some(ref mut story) = stories.get_mut(&ident) {
		let local_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
		println!("{local_time}: [story] cache hit:  {ident}");
		story.requests += 1;
		Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(story_template(story)))
	} else {
		drop(stories);
		let local_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
		println!("{local_time}: [story] cache miss: {ident}");
		let story = request_story(ident, &data.api).await?;
		let mut stories = data.stories.write().map_err(|_| "Failed to lock data")?;
		stories.insert(ident, story.clone());
		Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(story_template(&story)))
	}
}

#[get("/oembed/{type}/{id}")]
async fn oembed_story(
	path: web::Path<(String, String)>, data: web::Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
	let endpoint = path.clone().0;
	let ident = path.into_inner().1.parse::<u32>().unwrap();
	match endpoint.as_str() {
		"story" => {}
		"user" => {}
		"blog" => {}
		"error" => {}
		_ => {}
	}
	let stories = data.stories.read().map_err(|_| "Failed to lock data")?;
	if let Some(story) = stories.get(&ident) {
		Ok(HttpResponse::Ok()
			.content_type("application/json+oembed")
			.json(story.o_embed.clone()))
	} else {
		Ok(HttpResponse::NotFound().finish())
	}
}

#[get("/user/{id:.*}")]
async fn get_user(
	path: web::Path<String>, data: web::Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
	let ident = path.into_inner();
	let ident = ident.split('/').collect::<Vec<_>>();
	let ident = ident.first().unwrap();
	let ident = ident.parse::<u32>().unwrap();
	let mut users = data.users.write().map_err(|_| "Failed to lock data")?;
	if let Some(ref mut user) = users.get_mut(&ident) {
		let local_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
		println!("{local_time}: [user]  cache hit:  {ident}");
		user.requests += 1;
		Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(user_template(user)))
	} else {
		drop(users);
		let local_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
		println!("{local_time}: [user]  cache miss: {ident}");
		let user = request_user(ident, &data.api).await?;
		let mut users = data.users.write().map_err(|_| "Failed to lock data")?;
		users.insert(ident, user.clone());
		Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(user_template(&user)))
	}
}

#[get("/blog/{id:.*}")]
async fn get_blog(
	path: web::Path<String>, data: web::Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
	let ident = path.into_inner();
	let ident = ident.split('/').collect::<Vec<_>>();
	let ident = ident.first().unwrap();
	let ident = ident.parse::<u32>().unwrap();
	let mut blogs = data.blogs.write().map_err(|_| "Failed to lock data")?;
	if let Some(ref mut blog) = blogs.get_mut(&ident) {
		let local_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
		println!("{local_time}: [blog]  cache hit:  {ident}");
		blog.requests += 1;
		let users = data.users.read().map_err(|_| "Failed to lock data")?;
		let user = if let Some(user) = users.get(&blog.author_id) {
			user.clone()
		} else {
			request_user(blog.author_id, &data.api).await?
		};
		Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(blog_template(blog, &user)))
	} else {
		drop(blogs);
		let local_time = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
		println!("{local_time}: [blog]  cache miss: {ident}");
		let blog = request_blog(ident, &data.api).await?;
		let users = data.users.read().map_err(|_| "Failed to lock data")?;
		let user = if let Some(user) = users.get(&blog.author_id) {
			user.clone()
		} else {
			request_user(blog.author_id, &data.api).await?
		};
		let mut blogs = data.blogs.write().map_err(|_| "Failed to lock data")?;
		blogs.insert(ident, blog.clone());
		Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(blog_template(&blog, &user)))
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
		users: Arc::new(RwLock::new(HashMap::<u32, User>::new())),
		blogs: Arc::new(RwLock::new(HashMap::<u32, Blog>::new())),
	};

	let state_clone = app_data.clone();

	// Seconds between garbage collection.
	const GC: u64 = 3600;
	// Milliseconds to keep a story cached.
	const TTL: u128 = 86_400_000;

	tokio::task::spawn(async move {
		loop {
			tokio::time::sleep(Duration::from_secs(GC)).await;
			let time = unix_time().unwrap() - TTL;
			let local_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
			let stories = story_cleanup(state_clone.stories.clone(), time).unwrap();
			let users = user_cleanup(state_clone.users.clone(), time).unwrap();
			let blogs = blog_cleanup(state_clone.blogs.clone(), time).unwrap();
			println!(
				"{local_time} <- (requests, dropped, remaining) -> {stories}, {users}, {blogs}"
			)
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
			.service(oembed_story)
			.service(get_user)
			.service(get_blog)
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
				sleep(start_time, request.interval).await?;
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

async fn request_story(id: u32, api: &FimficRequest) -> Result<Story, Box<dyn std::error::Error>> {
	let fimfic = format!("https://www.fimfiction.net/api/v2/stories/{id}");
	let response = handle_request(api, &fimfic).await.unwrap();
	let api = response.json::<StoryApi>().await.unwrap();
	let author = api.included.iter().find_map(|author| {
		if author.id == api.data.relationships.author.data.id {
			let author = Author {
				url: author.meta.url.clone(),
				name: author.attributes.name.replace('"', "&quot;"),
			};
			Some(author)
		} else {
			None
		}
	});
	let story = api.data;
	let title = story.attributes.title.replace('"', "&quot;");
	let story = Story {
		id: story.id.parse::<u32>().unwrap(),
		link: story.meta.url,
		title: title.clone(),
		requests: 1,
		color: story.attributes.color.hex,
		author: author.clone().unwrap(),
		o_embed: create_o_embed(title, author, false),
		timestamp: unix_time().unwrap(),
		short_description: story.attributes.short_description.replace('"', "&quot;"),
		cover_medium_url: story.attributes.cover_image.map(|cover| cover.medium),
	};
	Ok(story)
}

async fn request_user(id: u32, api: &FimficRequest) -> Result<User, Box<dyn std::error::Error>> {
	let fimfic = format!("https://www.fimfiction.net/api/v2/users/{id}");
	let response = handle_request(api, &fimfic).await.unwrap();
	let api = response.json::<UserApi>().await.unwrap();
	let image = (!api.data.attributes.avatar.r64.ends_with("none_64.png"))
		.then_some(api.data.attributes.avatar.r256);
	let re = LazyLock::new(|| Regex::new(r"\[[^]]+\]").unwrap());
	let name = api.data.attributes.name.replace('"', "&quot;");
	let user = User {
		link: api.data.meta.url,
		name: name.clone(),
		requests: 1,
		color: api.data.attributes.color.hex,
		o_embed: create_o_embed(name, None, false),
		timestamp: unix_time().unwrap(),
		bio_bbcode: re
			.replace_all(&api.data.attributes.bio, "")
			.to_string()
			.replace('"', "&quot;"),
		profile_pic_256_url: image,
	};
	Ok(user)
}

async fn request_blog(id: u32, api: &FimficRequest) -> Result<Blog, Box<dyn std::error::Error>> {
	let fimfic = format!(
		"https://www.fimfiction.net/api/v2/blog-posts/{id}?fields[blog_post]=title,date_posted,content,num_views,num_comments,author,tagged_story"
	);
	let response = handle_request(api, &fimfic).await.unwrap();
	let api = response.json::<BlogApi>().await?;
	let story_id = (api.data.relationships.tagged_story.data.id != "0")
		.then_some(api.data.relationships.tagged_story.data.id.parse()?);
	let re = LazyLock::new(|| Regex::new(r"\[[^]]+\]").unwrap());
	let content = re
		.replace_all(&api.data.attributes.content, "")
		.to_string()
		.replace('"', "&quot;");
	let mut text = vec![];
	let mut chars = 0;
	for line in content.lines() {
		if chars + line.len() < 512 - 1 {
			text.push(line);
			chars += line.len();
		} else {
			break;
		}
	}
	let title = api.data.attributes.title;
	let blog = Blog {
		link: api.data.meta.url,
		title: title.clone(),
		requests: 1,
		author_id: api.data.relationships.author.data.id.parse()?,
		o_embed: create_o_embed(title, None, false),
		timestamp: unix_time().unwrap(),
		story_id,
		content_bbcode: text.join("\n"),
		date_published: api.data.attributes.date_posted,
	};
	Ok(blog)
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

fn story_template(story: &Story) -> String {
	match &story.cover_medium_url {
		Some(cover) => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{title}" />
		<meta property="og:description" content="{}" />
		<meta property="og:image" content="{}" />
		<meta property="og:url" content="{link}" />
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
			link = story.link,
		),
		None => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{title}" />
		<meta property="og:description" content="{}" />
		<meta property="og:url" content="{link}" />
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
			link = story.link,
		),
	}
}

fn user_template(user: &User) -> String {
	match &user.profile_pic_256_url {
		Some(image) => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{name}" />
		<meta property="og:description" content="{}" />
		<meta property="og:image" content="{}" />
		<meta property="og:url" content="{link}" />
		<meta property="og:type" content="profile" />
		<meta property="profile:username" content="{name}" />
		<meta property="og:site_name" content="Fimfiction" />
		<meta property="twitter:site" content="fimfiction" />
		<meta property="twitter:card" content="summary" />
	</head>
	<body></body>
	</html>"#,
			user.color,
			user.bio_bbcode,
			image,
			name = user.name,
			link = user.link
		),
		None => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{name}" />
		<meta property="og:description" content="{}" />
		<meta property="og:url" content="{link}" />
		<meta property="og:type" content="profile" />
		<meta property="profile:username" content="{name}" />
		<meta property="og:site_name" content="Fimfiction" />
		<meta property="twitter:site" content="fimfiction" />
		<meta property="twitter:card" content="summary" />
	</head>
	<body></body>
	</html>"#,
			user.color,
			user.bio_bbcode,
			name = user.name,
			link = user.link
		),
	}
}

fn blog_template(blog: &Blog, user: &User) -> String {
	match &user.profile_pic_256_url {
		Some(image) => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{}" />
		<meta property="og:description" content="{}" />
		<meta property="og:image" content="{}" />
		<meta property="og:url" content="{link}" />
		<meta property="og:type" content="article" />
		<meta property="article:author" content="{}" />
		<meta property="article:published_time" content="{}" />
		<meta property="og:site_name" content="Fimfiction" />
		<meta property="twitter:site" content="fimfiction" />
		<meta property="twitter:card" content="summary" />
	</head>
	<body></body>
	</html>"#,
			user.color,
			blog.title,
			blog.content_bbcode,
			image,
			user.link,
			blog.date_published,
			link = blog.link
		),
		None => format!(
			r#"<!DOCTYPE html>
	<html lang="en">
	<head>
		<meta name="theme-color" content="\#{}" />
		<link rel="canonical" href="{link}" />
		<meta http-equiv="refresh" content="0;url={link}" />
		<meta property="og:title" content="{}" />
		<meta property="og:description" content="{}" />
		<meta property="og:url" content="{link}" />
		<meta property="og:type" content="article" />
		<meta property="article:author" content="{}" />
		<meta property="article:published_time" content="{}" />
		<meta property="og:site_name" content="Fimfiction" />
		<meta property="twitter:site" content="fimfiction" />
		<meta property="twitter:card" content="summary" />
	</head>
	<body></body>
	</html>"#,
			user.color,
			blog.title,
			blog.content_bbcode,
			user.link,
			blog.date_published,
			link = blog.link
		),
	}
}
