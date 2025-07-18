use crate::fimfiction_api::error::FimficError;
use crate::fimfiction_api::tag::TagData;
use crate::structs::{Color, Cover, Parameters};
use chrono::{DateTime, FixedOffset};
use pony::http::{Request, api_get_request};
use rand::Rng;
use regex::Regex;
use serde::de::DeserializeOwned;
use sqlx::{Pool, Postgres, query};
use std::{collections::HashMap, error::Error, sync::LazyLock};

pub fn parse_id(path: &str) -> Result<i32, Box<dyn Error>> {
	let binding = path.to_string();
	let id = binding.split('/').collect::<Vec<_>>();
	let id = id.first().expect("First element will always be present.");
	match id.parse::<i32>() {
		Ok(id) => Ok(id),
		Err(_) => Err(format!("Failed to parse id as integer: {id}").into()),
	}
}

pub fn parse_second_id(path: &str) -> Option<i32> {
	let binding = path.to_string();
	let binding = binding.split('/').collect::<Vec<_>>();
	if binding.len() >= 3 || binding.len() == 2 && path.ends_with("/") {
		let id = binding.get(1);
		id.and_then(|id| id.parse::<i32>().ok())
	} else {
		None
	}
}

pub fn check_slash(path: &mut String, id: i32) {
	if !path.starts_with(&format!("{id}/")) {
		*path = format!("{path}/");
	}
}

pub async fn parse_embed_parameters(
	path: &mut String, queries: HashMap<String, String>, db: &Pool<Postgres>,
) -> (Parameters, String) {
	let mut params = Parameters::default();
	let mut errors = Vec::new();
	for (key, value) in queries {
		match key.to_lowercase().as_str() {
			"cover" | "image" => parse_cover(&mut params, &mut errors, value),
			"color" | "colour" => parse_color(&mut params, &mut errors, db, value).await,
			"refresh" => parse_bool(value, &mut params.refresh, &mut errors, &key),
			"stats" => parse_bool(value, &mut params.stats, &mut errors, &key),
			"tags" => parse_bool(value, &mut params.tags, &mut errors, &key),
			"comment" => parse_comment(path, &mut errors, value),
			_ => parse_error(&mut errors, key),
		}
	}
	(params, errors.join(", "))
}

fn parse_cover(params: &mut Parameters, errors: &mut Vec<String>, value: String) {
	let cover = Cover::try_from(value);
	match cover {
		Ok(cover) => params.cover = Some(cover),
		Err(err) => errors.push(err.to_string()),
	}
}

pub async fn parse_color(
	params: &mut Parameters, errors: &mut Vec<String>, db: &Pool<Postgres>, value: String,
) {
	let color = Color::from(value);
	if let Color::Custom(color) = color {
		let db_color = query!("SELECT color FROM Colors WHERE name = $1 LIMIT 1;", color)
			.fetch_optional(db)
			.await
			.unwrap_or_default();
		if let Some(color) = db_color {
			params.color = Some(Color::Custom(color.color));
		} else if color.len() == 6 {
			params.color = color
				.as_bytes()
				.iter()
				.all(|hex| hex.is_ascii_hexdigit())
				.then_some(Color::Custom(color.to_string()));
		} else {
			errors.push(format!("Unsupported color option: {color}"));
			params.color = None;
		}
	} else {
		params.color = Some(color);
	}
}

fn parse_bool(text: String, value: &mut bool, errors: &mut Vec<String>, key: &str) {
	match text.to_lowercase().as_str() {
		"false" | "0" | "no" | "n" | "f" => *value = false,
		"true" | "1" | "yes" | "y" | "t" => *value = true,
		_ => {
			errors.push(format!("Unsupported {key} value: {value}"));
		}
	}
}

fn parse_comment(path: &mut String, errors: &mut Vec<String>, value: String) {
	if path.contains("#comment/") {
		errors.push(format!("Duplicate comment: {value}"));
	} else {
		*path = format!("{path}#comment/{value}");
	}
}

fn parse_error(errors: &mut Vec<String>, key: String) {
	errors.push(format!("Unsupported option: {key}"));
}

pub async fn parse_fimfic_response<T: DeserializeOwned>(
	api: &Request, url: &str,
) -> Result<T, Box<dyn Error>> {
	let response = api_get_request(api, url)
		.await
		.map_err(|_| "FixFiction Error: API request error")?;
	let body = response
		.bytes()
		.await
		.map_err(|_| "FixFiction Error: reading Fimfiction API response")?;
	let api = serde_json::from_slice::<T>(&body);
	match api {
		Ok(api) => Ok(api),
		Err(_) => {
			let error = serde_json::from_slice::<FimficError<i32>>(&body)
				.map_err(|_| "FixFiction Error: API deserialization error")?;
			let error = error
				.errors
				.first()
				.ok_or("Fimfiction API Error: no error provided")?;
			Err(format!("Fimfiction API Error: {} – {}", error.code, error.title).into())
		}
	}
}

pub fn trim_content(content: String, clean: bool) -> String {
	let content = match clean {
		true => clean_content(content),
		false => content,
	};
	let mut text = vec![];
	let mut chars = 0;
	for line in content.lines() {
		let line = line.trim();
		if chars == 0 && line.len() > 512 {
			return line.to_string();
		} else if line.is_empty() {
			continue;
		} else if chars + line.len() < 512 {
			text.push(line);
			chars += line.len() + 2;
		} else {
			break;
		}
	}
	text.join("\n\n")
}

pub fn clean_content(content: String) -> String {
	let re = LazyLock::new(|| {
		Regex::new(
			r":[a-z]{1,20}[0-9]?:|\[icon\].*\[/icon\]|\[img\].*\[/img\]|\[embed\].*\[/embed\]|\[[^]]+\]|https?:\/\/[A-Za-z0-9]{1,256}\.[A-Za-z0-9]{1,256}\.[A-Za-z0-9]{1,256}(\/.*)?",
		)
		.unwrap()
	});
	re.replace_all(&content, "").to_string().replace('⠀', "")
}

pub fn map_cover(link: Option<String>) -> Option<String> {
	link.map(|link| format!("{link}-full"))
}

pub fn map_picture(link: Option<String>) -> Option<String> {
	link.map(|link| format!("{link}-512"))
}

pub fn parse_date(date: String, name: &str) -> Result<DateTime<FixedOffset>, Box<dyn Error>> {
	Ok(DateTime::parse_from_rfc3339(&date)
		.map_err(|_| format!("FixFiction Error: failed to parse {name} date"))?)
}

pub fn map_tags(tags: Vec<&TagData<i32>>) -> String {
	tags.iter()
		.map(|tag| tag.attributes.name.clone())
		.collect::<Vec<_>>()
		.join(", ")
}

#[macro_export]
macro_rules! get_variant {
	($vec:expr, $ty:path) => {{
		$vec.iter().find_map(|inc| {
			if let $ty(data) = inc {
				Some(data)
			} else {
				None
			}
		})
	}};
}

#[macro_export]
macro_rules! get_variants {
	($vec:expr, $ty:path) => {{
		$vec.iter().filter_map(|inc| {
			if let $ty(data) = inc {
				Some(data)
			} else {
				None
			}
		})
	}};
}

#[macro_export]
macro_rules! check_recache {
	($item:expr, $recache:expr, $app:expr) => {{
		match $recache {
			true => $item.filter(|item| {
				Utc::now()
					.checked_sub_signed(TimeDelta::seconds($app.cache_recache_age))
					.is_some_and(|max_age| item.date_cached >= max_age)
			}),
			false => $item,
		}
	}};
}

pub fn get_color(id: Option<i32>) -> String {
	let colors = ["cc9cdf", "faba62", "faf5ab", "f5b7d0", "9bdbf5", "eaeef0"];
	match id {
		Some(id) => colors[(id % 6) as usize].to_string(),
		None => {
			let mut rng = rand::rng();
			let idx = rng.random_range(0..=5) as usize;
			colors[idx].to_string()
		}
	}
}
