use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use chrono::Local;
use pony::fimfiction_api::story::StoryApi;
use pony::fimfiction_api::user::UserApi;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::{Arc, RwLock};
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
	author_name: String,
	author_url: String,
	cache_age: u32,
	html: String,
}

#[derive(Debug, Clone)]
struct User {
	id: u32,
	link: String,
	name: String,
	requests: u32,
	color: String,
	timestamp: u128,
	bio_bbcode: String,
	profile_pic_256_url: Option<String>,
}

#[get("/story/{id:.*}")]
async fn get_story(
	path: web::Path<String>, api: web::Data<Arc<FimficRequest>>,
	data: web::Data<Arc<RwLock<HashMap<u32, Story>>>>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
	let ident = path.into_inner();
	let ident = ident.split('/').collect::<Vec<_>>();
	let ident = ident.first().unwrap();
	let ident = ident.parse::<u32>().unwrap();
	let mut stories = data.write().map_err(|_| "Failed to lock data")?;
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
		let fimfic = format!("https://www.fimfiction.net/api/v2/stories/{ident}");
		let response = handle_request(&api, &fimfic).await.unwrap();
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
		let embed = OEmbed {
			r#type: String::from(""),
			version: 1,
			provider_name: String::from("Fimfiction"),
			provider_url: String::from("https://www.fimfiction.net/"),
			title: story.attributes.title.replace('"', "&quot;"),
			author_name: author.clone().unwrap().name,
			author_url: author.clone().unwrap().url,
			cache_age: 86_400,
			html: String::default(),
		};
		let story = Story {
			id: story.id.parse::<u32>().unwrap(),
			link: story.meta.url,
			title: embed.title.clone(),
			requests: 1,
			color: story.attributes.color.hex,
			author: author.unwrap(),
			o_embed: embed,
			timestamp: unix_time().unwrap(),
			short_description: story.attributes.short_description.replace('"', "&quot;"),
			cover_medium_url: story.attributes.cover_image.map(|cover| cover.medium),
		};
		let mut stories = data.write().map_err(|_| "Failed to lock data")?;
		stories.insert(story.id, story.clone());
		Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(story_template(&story)))
	}
}

#[get("/oembed/story/{id}")]
async fn oembed_story(
	path: web::Path<String>, data: web::Data<Arc<RwLock<HashMap<u32, Story>>>>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
	let ident = path.into_inner().parse::<u32>().unwrap();
	let stories = data.read().map_err(|_| "Failed to lock data")?;
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
	path: web::Path<String>, api: web::Data<Arc<FimficRequest>>,
	data: web::Data<Arc<RwLock<HashMap<u32, User>>>>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
	let ident = path.into_inner();
	let ident = ident.split('/').collect::<Vec<_>>();
	let ident = ident.first().unwrap();
	let ident = ident.parse::<u32>().unwrap();
	let mut users = data.write().map_err(|_| "Failed to lock data")?;
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
		let fimfic = format!("https://www.fimfiction.net/api/v2/users/{ident}");
		let response = handle_request(&api, &fimfic).await.unwrap();
		let api = response.json::<UserApi>().await.unwrap();
		let image = (!api.data.attributes.avatar.r64.ends_with("none_64.png"))
			.then_some(api.data.attributes.avatar.r256);
		let user = User {
			id: ident,
			link: api.data.meta.url,
			name: api.data.attributes.name.replace('"', "&quot;"),
			requests: 1,
			color: api.data.attributes.color.hex,
			timestamp: unix_time().unwrap(),
			bio_bbcode: api.data.attributes.bio.replace('"', "&quot;"),
			profile_pic_256_url: image,
		};
		Ok(HttpResponse::Ok()
			.content_type("text/html; charset=utf-8")
			.body(user_template(&user)))
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
	let api = Arc::new(api);

	let stories = Arc::new(RwLock::new(HashMap::<u32, Story>::new()));
	let stories_clone = stories.clone();

	let users = Arc::new(RwLock::new(HashMap::<u32, User>::new()));
	let users_clone = users.clone();

	// Seconds between garbage collection.
	const GC: u64 = 3600;
	// Milliseconds to keep a story cached.
	const TTL: u128 = 86_400_000;

	tokio::task::spawn(async move {
		loop {
			tokio::time::sleep(Duration::from_secs(GC)).await;
			let time = unix_time().unwrap();
			let local_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
			let mut stories = stories_clone.write().expect("Failed to lock data");
			let count = stories.len();
			let requests = stories.iter().map(|(_, story)| story.requests).sum::<u32>();
			stories.retain(|_, story| story.timestamp + TTL > time);
			let remaining = stories.len();
			let dropped = count - remaining;
			println!(
				"{local_time}: [story] Requests - {requests}, Stories - {count}, Dropped - {dropped}, Remaining - {remaining}"
			);

			let mut users = users_clone.write().expect("Failed to lock data");
			let count = users.len();
			let requests = users.iter().map(|(_, user)| user.requests).sum::<u32>();
			users.retain(|_, user| user.timestamp + TTL > time);
			let remaining = users.len();
			let dropped = count - remaining;
			println!(
				"{local_time}: [user]  Requests - {requests}, Stories - {count}, Dropped - {dropped}, Remaining - {remaining}"
			);
		}
	});

	HttpServer::new(move || {
		App::new()
			.app_data(Data::new(api.clone()))
			.app_data(Data::new(stories.clone()))
			.app_data(Data::new(users.clone()))
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
