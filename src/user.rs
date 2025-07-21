use crate::check_recache;
use crate::database::{get_user, insert_user};
use crate::fimfiction_api::user::UserApi;
use crate::html_template::embed_html_template;
use crate::structs::{AppState, Color, Cover, EmbedData, Parameters, User};
use crate::utility::{get_color, map_picture, parse_fimfic_response};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};
use std::error::Error;

pub async fn request_user(id: i32, app: &AppState, recache: bool) -> Result<User, Box<dyn Error>> {
	let user = get_user(id, &app.db).await?;
	let user = check_recache!(user, recache, app);
	match user {
		Some(user) => Ok(user),
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/users/{id}");
			let api = parse_fimfic_response::<UserApi<i32>>(&app.api, &fimfic).await?;
			insert_user(Some(id), &api.data, &app.db).await
		}
	}
}

pub fn user_html_template(
	user: User, parameters: Parameters, link: String, errors: String,
) -> String {
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(user.id))),
			_ => Some(user.color_hex),
		},
		None => Some(user.color_hex),
	};
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::None => None,
			Cover::User | Cover::Founder => map_picture(user.profile_pic_url),
			Cover::Story => map_picture(user.profile_pic_url),
		},
		None => map_picture(user.profile_pic_url),
	};
	let site_name = if parameters.stats {
		{
			let time = user.date_joined.format("%a %b %e %Y").to_string();
			format!(
				"Fimfiction - Joined: {time} 📅\nStories: {} 📚 Blogs: {} 📑 Followers: {} 👥",
				format_number_unit_metric(user.stories as f64, FormatType::MetricPrefix, 1)
					.unwrap(),
				format_number_unit_metric(user.blogs as f64, FormatType::MetricPrefix, 1).unwrap(),
				format_number_unit_metric(user.followers as f64, FormatType::MetricPrefix, 1)
					.unwrap(),
			)
		}
	} else {
		"Fimfiction".to_string()
	};
	let data = EmbedData {
		title: user.name,
		description: user.bio,
		link,
		color,
		cover,
		site_name,
		site_url: String::from("https://www.fimfiction.net/"),
		errors,
		user_name: None,
		user_link: None,
		html_comment: None,
		open_graph_type: String::from("profile"),
		open_graph_property: Some(String::from("profile:username")),
	};
	embed_html_template(data)
}
