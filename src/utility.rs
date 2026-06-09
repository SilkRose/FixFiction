//! Various utility functions for use in other modules.

use crate::fimfiction_api::error::FimficError;
use crate::structs::{Color, Cover, Parameters, Tag};
use chrono::{DateTime, FixedOffset};
use pony::http::{Request, api_get_request};
use pony::log::{FileLimit, LogLevel, Logger};
use rand::RngExt;
use regex::Regex;
use serde::de::DeserializeOwned;
use sqlx::{Pool, Postgres, query};
use std::collections::HashMap;
use std::error::Error;
use std::iter;
use std::sync::LazyLock;
use url::form_urlencoded;

/// Logger that prints to the console and a file
///
/// #### Panics
///
/// Panics if it fails to set the file.
pub(crate) static LOG: LazyLock<Logger> = LazyLock::new(|| {
	Logger::new(LogLevel::Debug)
		.set_file("./logs", LogLevel::Debug, FileLimit::Lines(1_000), 20)
		.expect("Should never fail")
});

/// Parses Fimfiction IDs to [i32]
///
/// #### Panics
///
/// Panics if the first segment doesn't exist.
pub(crate) fn parse_id(path: &str) -> Result<i32, Box<dyn Error>> {
	let binding = path.to_string();
	let id = binding.split('/').collect::<Vec<_>>();
	let id = id.first().expect("First element will always be present.");
	match id.parse::<i32>() {
		Ok(id) => Ok(id),
		Err(_) => Err(format!("Failed to parse id as integer: {id}").into()),
	}
}

/// Parses Fimfiction chapter ID to [i32]
pub(crate) fn parse_chapter_number(path: &str) -> Option<i32> {
	let binding = path.to_string();
	let binding = binding.split('/').collect::<Vec<_>>();
	if binding.len() >= 3 || binding.len() == 2 && path.ends_with("/") {
		let id = binding.get(1);
		id.and_then(|id| id.parse::<i32>().ok())
	} else {
		None
	}
}

/// Parses Fimfiction thread ID to [i32]
pub(crate) fn parse_thread_id(path: &str) -> Option<i32> {
	let parts: Vec<_> = path.split('/').collect();
	if parts.get(2)? != &"thread" {
		return None;
	}
	parts.get(3)?.parse().ok()
}

/// Adds a missing forward slash to the URL if it's missing
pub(crate) fn check_slash(path: &mut String, id: i32) {
	if !path.starts_with(&format!("{id}/")) {
		*path = format!("{path}/");
	}
}

/// Adds a missing forward slash to the URL if it's missing
pub(crate) fn check_thread_slash(path: &mut String, id: i32) {
	if path.ends_with(&format!("/thread/{id}")) {
		*path = format!("{path}/");
	}
}

/// Parses a [HashMap<String, String>] into [Parameters]
pub(crate) async fn parse_embed_parameters(
	path: &mut String, queries: HashMap<String, String>, db: &Pool<Postgres>,
) -> (Parameters, Vec<String>) {
	let mut params = Parameters::default();
	let mut errors = Vec::new();
	let mut comment = None;
	for (key, value) in queries {
		match key.to_lowercase().as_str() {
			"cover" | "image" => parse_cover(&mut params, &mut errors, value),
			"color" | "colour" => parse_color(&mut params, &mut errors, db, value).await,
			"refresh" | "renew" => parse_bool(value, &mut params.refresh, &mut errors, &key),
			"stats" | "info" => parse_bool(value, &mut params.stats, &mut errors, &key),
			"tags" | "tag" => parse_bool(value, &mut params.tags, &mut errors, &key),
			"comment" => comment = Some(value),
			_ => append_query(path, &key, &value),
		}
	}
	if let Some(comment_id) = comment {
		*path = format!("{path}#comment/{comment_id}");
	}
	(params, errors)
}

/// Converts a string into a [Cover]
pub(crate) fn parse_cover(params: &mut Parameters, errors: &mut Vec<String>, value: String) {
	let cover = match value.to_lowercase().as_str() {
		"founder" => Ok(Cover::Founder),
		"story" => Ok(Cover::Story),
		"user" => Ok(Cover::User),
		"none" => Ok(Cover::None),
		_ => Err(format!("Unsupported cover option: {value}")),
	};
	match cover {
		Ok(cover) => params.cover = Some(cover),
		Err(err) => errors.push(err.to_string()),
	}
}

/// Converts a string into a [Color]
pub(crate) async fn parse_color(
	params: &mut Parameters, errors: &mut Vec<String>, db: &Pool<Postgres>, value: String,
) {
	let color = match value.to_lowercase().as_str() {
		"ran" | "random" => Color::Random,
		"mod" | "modulo" => Color::Modulo,
		"founder" => Color::Founder,
		"story" => Color::Story,
		"user" => Color::User,
		"none" => Color::None,
		_ => Color::Custom(value.to_lowercase()),
	};
	if let Color::Custom(color) = color {
		let db_color = query!("SELECT color FROM Colors WHERE name = $1 LIMIT 1;", color)
			.fetch_optional(db)
			.await
			.unwrap_or_default();
		if let Some(color) = db_color {
			params.color = Some(Color::Custom(color.color));
		} else if matches!(color.len(), 1 | 2 | 6) {
			params.color = color
				.as_bytes()
				.iter()
				.all(|hex| hex.is_ascii_hexdigit())
				.then_some(Color::Custom(color.repeat(6 / color.len())));
		} else if color.len() == 3 {
			params.color = color
				.as_bytes()
				.iter()
				.all(|&hex| hex.is_ascii_hexdigit())
				.then(|| Color::Custom(color.chars().flat_map(|c| iter::repeat_n(c, 2)).collect()));
		} else {
			errors.push(format!("Unsupported color option: {color}"));
			params.color = None;
		}
	} else {
		params.color = Some(color);
	}
}

/// Parses a [bool] from a [String] with variable accepted inputs
pub(crate) fn parse_bool(text: String, value: &mut bool, errors: &mut Vec<String>, key: &str) {
	match text.to_lowercase().as_str() {
		"false" | "0" | "no" | "n" | "f" => *value = false,
		"true" | "1" | "yes" | "y" | "t" => *value = true,
		_ => {
			errors.push(format!("Unsupported {key} value: {text}"));
		}
	}
}

/// Appends unknown query parameters onto the target URL
pub(crate) fn append_query(path: &mut String, key: &str, value: &str) {
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair(key, value);
	if path.contains('?') {
		*path = format!("{path}&{}", encode.finish());
	} else {
		*path = format!("{path}?{}", encode.finish());
	}
}

/// Sends a request and parses the response from the Fimfiction API
pub(crate) async fn parse_fimfic_response<T: DeserializeOwned>(
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

/// Trims content to improve the look of embeds
pub(crate) fn trim_content(content: String, clean: bool) -> String {
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

/// Cleans content to improve the look of embeds
///
/// #### Panics
///
/// Panics if the regex fails to compile.
pub(crate) fn clean_content(content: String) -> String {
	let re = LazyLock::new(|| {
		Regex::new(
			r":[a-z]{1,20}[0-9]?:|\[icon\].*\[/icon\]|\[img\].*\[/img\]|\[embed\].*\[/embed\]|\[[^]]+\]|https?:\/\/[A-Za-z0-9]{1,256}\.[A-Za-z0-9]{1,256}\.[A-Za-z0-9]{1,256}(\/.*)?",
		)
		.unwrap()
	});
	re.replace_all(&content, "").to_string().replace('⠀', "")
}

/// Selects which story cover size to embed
pub(crate) fn map_cover(link: Option<String>) -> Option<String> {
	link.map(|link| format!("{link}-full"))
}

/// Selects which profile picture size to embed
pub(crate) fn map_picture(link: Option<String>) -> Option<String> {
	link.map(|link| format!("{link}-512"))
}

/// Converts a RFC3339 date string into a [DateTime]
pub(crate) fn parse_date(
	date: String, name: &str,
) -> Result<DateTime<FixedOffset>, Box<dyn Error>> {
	Ok(DateTime::parse_from_rfc3339(&date)
		.map_err(|_| format!("FixFiction Error: failed to parse {name} date"))?)
}

/// Shortens tag names and joins them with a comma
pub(crate) fn map_tags(tags: &[Tag]) -> String {
	tags.iter()
		.map(|tag| SHORT_TAGS.get(&tag.id).copied().unwrap_or(&tag.name))
		.collect::<Vec<_>>()
		.join(", ")
}

/// Fimfiction tag name shorthands
static SHORT_TAGS: LazyLock<HashMap<i32, &str>> = LazyLock::new(|| {
	let mut tags = HashMap::new();
	tags.insert(4, "MLP FiM");
	tags.insert(6, "Twilight");
	tags.insert(7, "Rainbow");
	tags.insert(8, "Pinkie"); // Best pony!
	tags.insert(16, "Celestia");
	tags.insert(17, "Luna");
	tags.insert(21, "Big Mac");
	tags.insert(40, "Opal");
	tags.insert(44, "Derpy");
	tags.insert(47, "Vinyl");
	tags.insert(48, "OC");
	tags.insert(64, "Flim & Flam");
	tags.insert(69, "Dinky");
	tags.insert(71, "Cadence");
	tags.insert(73, "Mane 6 (G4)");
	tags.insert(74, "CMC");
	tags.insert(77, "Chrysalis");
	tags.insert(79, "Flitter & Cloudchaser");
	tags.insert(93, "Sunset");
	tags.insert(98, "Brad");
	tags.insert(113, "Adagio");
	tags.insert(114, "Sonata");
	tags.insert(115, "Aria");
	tags.insert(123, "Equestria Girls");
	tags.insert(128, "Starlight");
	tags.insert(136, "Starswirl");
	tags.insert(166, "Pinkie (EqG)"); // Best girl!
	tags.insert(169, "Rainbow (EqG)");
	tags.insert(177, "Sunset (Demon)");
	tags.insert(178, "Sci-Twi");
	tags.insert(180, "Nightmarity");
	tags.insert(211, "Mane 7 (EqG)");
	tags.insert(225, "2nd Person");
	tags.insert(236, "Sci-Fi");
	tags.insert(240, "AU");
	tags.insert(242, "MLP G4 Movie");
	tags.insert(243, "MLP Comic");
	tags.insert(516, "Hitch");
	tags.insert(517, "Izzy");
	tags.insert(518, "Sunny");
	tags.insert(528, "Pipp");
	tags.insert(529, "Zipp");
	tags.insert(531, "MLP G5");
	tags.insert(557, "Mane 5 (G5)");
	tags
});

/// Takes a [Vec] of an enum and gets a specific variant
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

/// Takes a [Vec] of an enum and gets all variants of a specific variant
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

/// Checks the cache time to see if its been long enough to re-fetch from Fimfiction's API
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

/// Returns a mane 6 coat hex-color.
/// Picks a color based on the ID modulo 6.
/// Picks a color at random if no ID is provided.
pub(crate) fn get_color(id: Option<i32>) -> String {
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

/// Inserts the error message for an unsupported cover option.
pub(crate) fn unsupported_cover_opt(
	errors: &mut Vec<String>, option: String, res: Option<String>,
) -> Option<String> {
	errors.push(format!("Unsupported cover option: {option}"));
	res
}

/// Inserts the error message for an unsupported color option.
pub(crate) fn unsupported_color_opt(
	errors: &mut Vec<String>, option: String, res: Option<String>,
) -> Option<String> {
	errors.push(format!("Unsupported color option: {option}"));
	res
}

/// Inserts the error message for an unsupported cover option.
/// Has an optional return for easy use in parameter handling.
pub(crate) fn unsupported_cover(
	errors: &mut Vec<String>, option: String, res: String,
) -> Option<String> {
	errors.push(format!("Unsupported cover option: {option}"));
	Some(res)
}

/// Inserts the error message for an unsupported color option.
/// Has an optional return for easy use in parameter handling.
pub(crate) fn unsupported_color(
	errors: &mut Vec<String>, option: String, res: String,
) -> Option<String> {
	errors.push(format!("Unsupported color option: {option}"));
	Some(res)
}
