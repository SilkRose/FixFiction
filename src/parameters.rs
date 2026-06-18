//! Parses and handles embed [Parameters]

use crate::database::Db;
use sqlx::query;
use std::collections::HashMap;
use std::{fmt, iter};
use url::form_urlencoded;

/// Embed parameter options
#[derive(Debug, Clone, Default)]
pub(crate) struct Parameters {
	pub(crate) cover: Option<Cover>,
	pub(crate) color: Option<Color>,
	pub(crate) refresh: bool,
	pub(crate) stats: bool,
	pub(crate) tags: bool,
}

/// Supported image options for embeds
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Cover {
	Founder,
	Story,
	User,
	None,
}

impl fmt::Display for Cover {
	/// Returns a string representation of a cover enum
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let text = match self {
			Cover::Founder => "founder",
			Cover::Story => "story",
			Cover::User => "user",
			Cover::None => "none",
		};
		write!(f, "{text}")
	}
}

/// Supported color options for embeds
#[derive(Debug, Clone)]
pub(crate) enum Color {
	Custom(String),
	Founder,
	Random,
	Modulo,
	Story,
	User,
	None,
}

impl fmt::Display for Color {
	/// Returns a string representation of a color enum
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let text = match self {
			Color::Custom(color) => color.as_str(),
			Color::Founder => "founder",
			Color::Random => "random",
			Color::Modulo => "modulo",
			Color::Story => "story",
			Color::User => "user",
			Color::None => "none",
		};
		write!(f, "{text}")
	}
}

/// Parses a [HashMap<String, String>] into [Parameters]
pub(crate) async fn parse_embed_parameters(
	path: &mut String, queries: HashMap<String, String>, db: &Db,
) -> (Parameters, Vec<String>) {
	let mut params = Parameters::default();
	let mut errors = Vec::new();
	for (key, value) in queries {
		match key.to_lowercase().as_str() {
			"cover" | "image" => parse_cover(&mut params, &mut errors, value),
			"color" | "colour" => parse_color(&mut params, &mut errors, db, value).await,
			"refresh" | "renew" => parse_bool(value, &mut params.refresh, &mut errors, &key),
			"stats" | "info" => parse_bool(value, &mut params.stats, &mut errors, &key),
			"tags" | "tag" => parse_bool(value, &mut params.tags, &mut errors, &key),
			_ => append_query(path, &key, &value),
		}
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
	params: &mut Parameters, errors: &mut Vec<String>, db: &Db, value: String,
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
			.fetch_optional(&db.pool)
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
