use crate::structs::{Color, Parameters};
use regex::Regex;
use sqlx::{Pool, Postgres, query};
use std::{error::Error, sync::LazyLock};

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

pub async fn parse_parameters(
	params: &mut Parameters, db: &Pool<Postgres>,
) -> Result<(), Box<dyn Error>> {
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
	Ok(())
}

pub fn trim_content(content: String, clean: bool) -> String {
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

pub fn clean_content(content: String) -> String {
	let re = LazyLock::new(|| Regex::new(r"\[[^]]+\]").unwrap());
	re.replace_all(&content, "")
		.to_string()
		.replace('"', "&quot;")
}
