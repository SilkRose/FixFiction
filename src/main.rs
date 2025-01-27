use self::structs::Api;
use actix_cors::Cors;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Response};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
	author: String,
	timestamp: u128,
	short_description: String,
	cover_medium_url: Option<String>,
}

#[get("/stories/{id}")]
async fn get_story(
	path: web::Path<String>, api: web::Data<Arc<FimficRequest>>,
	data: web::Data<Arc<Mutex<HashMap<u32, Story>>>>,
) -> Result<impl Responder, Box<dyn std::error::Error>> {
	let ident = path.into_inner();
	let ident = ident.split('/').collect::<Vec<_>>();
	let ident = ident.first().unwrap();
	let ident = ident.parse::<u32>().unwrap();
	let stories = data.lock().map_err(|_| "Failed to lock data")?;
	if let Some(story) = stories.get(&ident) {
		Ok(HttpResponse::Ok().body(""))
	} else {
		let fimfic = String::from("https://www.fimfiction.net/api/v2/stories?filter%5Bids%5D=571171,570257,565869,565515,562089");
		let test = handle_request(&api, &fimfic).await?;
		let api = test.json::<Api>().await?;
		println!("{:?}", api);

		Ok(HttpResponse::Found()
			.append_header((
				"Location",
				"https://www.fimfiction.net/story/571171/hearths-warming-homecoming",
			))
			.body("Story not found"))
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

	let cache_duration = 3_600;

	let stories = Arc::new(Mutex::new(HashMap::<u32, Story>::new()));

	tokio::task::spawn(async {
		loop {
			sleep(unix_time().unwrap(), Duration::from_secs(3600))
				.await
				.unwrap();
			// code for cleanup here?
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

async fn handle_request(request: &FimficRequest, url: &str) -> Result<Response, Box<dyn Error>> {
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
			Ok(Err(e)) => {
				println!("Request failed: {e}");
			}
			Err(e) => {
				println!("Request timed out: {e}");
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

async fn sleep(start_time: u128, interval: Duration) -> Result<(), Box<dyn Error>> {
	let current_time = unix_time()?;
	let elapsed_time = Duration::from_millis((current_time - start_time).try_into()?);
	if elapsed_time > interval {
		return Ok(());
	};
	tokio::time::sleep(interval - elapsed_time).await;
	Ok(())
}

fn unix_time() -> Result<u128, Box<dyn Error>> {
	Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
}

fn story_template(story: &Story) -> String {
	format!(
		r#"<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<meta http-equiv="refresh" content="0;url={link}">
	<meta property="og:title" content="{title}">
	<meta property="og:description" content="{}">
	<meta property="og:image" content="{}">
	<meta property="og:url" content="{link}">
	<meta property="og:type" content="book" />
	<meta property="book:author" content="{}" />
	<meta property="og:site_name" content="Fimfiction" />
	<meta property="twitter:site" content="fimfiction" />
	<meta property="twitter:card" content="summary" />
	<title>{title}- Fimfiction</title>
</head>
<body>
	<p>If you are not redirected, <a href="{link}">click here</a>.</p>
</body>
</html>"#,
		story.short_description,
		story.cover_medium_url.as_ref().unwrap(),
		story.author,
		title = story.title,
		link = story.link,
	)
}
