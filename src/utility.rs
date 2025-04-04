use crate::structs::{Color, Parameters};
use regex::Regex;
use sqlx::{Pool, Postgres, query};
use std::{error::Error, sync::LazyLock};

pub async fn parse_parameters(
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
