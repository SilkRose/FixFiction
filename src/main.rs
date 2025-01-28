use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Response};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use structs::{Api, ApiData};
use tokio::time::timeout;

pub mod structs;

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
	author: String,
	short_description: String,
	cover_medium_url: Option<String>,
}

#[get("/story/{id:.*}")]
async fn get_story(
	path: web::Path<String>, api: web::Data<Arc<FimficRequest>>,
	data: web::Data<Arc<Mutex<HashMap<u32, Story>>>>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
	let ident = path.into_inner();
	let ident = ident.split('/').collect::<Vec<_>>();
	let ident = ident.first().unwrap();
	let ident = ident.parse::<u32>().unwrap();
	let mut stories = data.lock().map_err(|_| "Failed to lock data")?;
	if let Some(ref mut story) = stories.get_mut(&ident) {
		story.requests += 1;
		Ok(HttpResponse::Ok().body(story_template(story)))
	} else {
		drop(stories);
		let fimfic = format!("https://www.fimfiction.net/api/v2/stories/{ident}");
		let response = handle_request(&api, &fimfic).await.unwrap();
		let api = response.json::<Api<ApiData>>().await.unwrap();
		let author = api.included.iter().find_map(|author| match author {
			structs::ApiIncluded::Tag(_) => None,
			structs::ApiIncluded::Author(included_author) => {
				if included_author.id == api.data.relationships.author.data.id {
					Some(included_author.meta.url.clone())
				} else {
					None
				}
			}
		});
		let story = api.data;
		let story = Story {
			id: story.id.parse::<u32>().unwrap(),
			link: story.meta.url,
			title: story.attributes.title,
			requests: 0,
			author: author.unwrap(),
			short_description: story.attributes.short_description,
			cover_medium_url: story.attributes.cover_image.map(|cover| cover.medium),
		};
		let mut stories = data.lock().map_err(|_| "Failed to lock data")?;
		stories.insert(story.id, story.clone());
		Ok(HttpResponse::Ok().body(story_template(&story)))
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
	let api_clone = Arc::clone(&api);

	let interval = 10;

	let stories = Arc::new(Mutex::new(HashMap::<u32, Story>::new()));
	let stories_clone = Arc::clone(&stories);

	tokio::task::spawn(async move {
		loop {
			sleep(unix_time().unwrap(), Duration::from_secs(interval))
				.await
				.unwrap();

			let story_ids: Vec<u32> = {
				let lock = stories_clone.lock();
				match lock {
					Ok(stories) => stories
						.iter()
						.filter(|(_, story)| story.requests > 1)
						.map(|(id, _)| *id)
						.collect(),
					Err(error) => {
						eprintln!("Failed to lock stories: {error}");
						return;
					}
				}
			};

			let mut stories: HashMap<u32, Story> = HashMap::new();
			for chunk in story_ids.chunks(100) {
				let fimfic = format!(
					"https://www.fimfiction.net/api/v2/stories?filter%5Bids%5D={}",
					chunk
						.iter()
						.map(|id| id.to_string())
						.collect::<Vec<_>>()
						.join(",")
				);

				let response = handle_request(&api_clone, &fimfic).await.unwrap();
				let api = response.json::<Api<Vec<ApiData>>>().await.unwrap();
				for story in api.data {
					let author = api.included.iter().find_map(|author| match author {
						structs::ApiIncluded::Tag(_) => None,
						structs::ApiIncluded::Author(included_author) => {
							if included_author.id == story.relationships.author.data.id {
								Some(included_author.meta.url.clone())
							} else {
								None
							}
						}
					});

					let story = Story {
						id: story.id.parse::<u32>().unwrap(),
						link: story.meta.url,
						title: story.attributes.title,
						requests: 0,
						author: author.unwrap(),
						short_description: story.attributes.short_description,
						cover_medium_url: story.attributes.cover_image.map(|cover| cover.medium),
					};
					stories.insert(story.id, story);
				}
			}
			let lock = stories_clone.lock();
			match lock {
				Ok(mut stories_lock) => {
					let dropped = stories_lock.len() - stories.len();
					println!(
						"{}: stories kept - {}, stories dropped - {dropped}",
						Utc::now(),
						stories.len()
					);
					*stories_lock = stories;
				}
				Err(error) => {
					eprintln!("Failed to lock stories: {error}");
					return;
				}
			}
		}
	});

	HttpServer::new(move || {
		App::new()
			.app_data(web::Data::new(api.clone()))
			.app_data(web::Data::new(stories.clone()))
			.wrap(
				Cors::default()
					.allow_any_origin()
					.allow_any_method()
					.allow_any_header()
					.max_age(3600),
			)
			.service(get_story)
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
	format!(
		r#"<!DOCTYPE html>
<html lang="en">
<head>
	<link rel="canonical" href="{link}" />
	<meta property="og:title" content="{title}" />
	<meta property="og:description" content="{}" />
	<meta property="og:image" content="{}" />
	<meta property="og:url" content="{link}" />
	<meta property="og:type" content="book" />
	<meta property="book:author" content="{}" />
	<meta property="og:site_name" content="Fimfiction" />
	<meta property="twitter:site" content="fimfiction" />
	<meta property="twitter:card" content="summary" />
	<meta http-equiv="refresh" content="0;url={link}" />
</head>
<body></body>
</html>"#,
		story.short_description,
		story.cover_medium_url.as_ref().unwrap(),
		story.author,
		title = story.title,
		link = story.link,
	)
}
