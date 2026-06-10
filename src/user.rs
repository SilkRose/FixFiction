//! Request a [User] and to format it in HTML.

use crate::check_recache;
use crate::database::{get_user, insert_user};
use crate::error::error_html_template;
use crate::fimfiction_api::user::UserApi;
use crate::html_template::embed_html_template;
use crate::structs::{AppState, Color, Cover, EmbedData, Parameters};
use crate::utility::{
	check_slash, get_color, map_picture, parse_embed_parameters, parse_fimfic_response, parse_id,
	unsupported_color, unsupported_cover_opt,
};
use actix_web::web::{Data, Path, Query};
use actix_web::{HttpResponse, Responder, get};
use chrono::{DateTime, TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

/// Fimfiction user data converted into a more usable structure
#[derive(Debug, Clone)]
pub(crate) struct User {
	pub(crate) id: i32,
	pub(crate) name: String,
	pub(crate) bio: String,
	pub(crate) link: String,
	pub(crate) followers: i32,
	pub(crate) stories: i32,
	pub(crate) blogs: i32,
	pub(crate) profile_pic_url: Option<String>,
	pub(crate) color_hex: String,
	pub(crate) date_joined: DateTime<Utc>,
	pub(crate) date_cached: DateTime<Utc>,
}

/// The `user/` endpoint.
///
/// Requests a user by ID.
#[get("/user/{id:.*}")]
async fn get_user_endpoint(
	path: Path<String>, queries: Query<HashMap<String, String>>, app: Data<Arc<AppState>>,
) -> Result<impl Responder, Box<dyn Error>> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let user_id = match parse_id(&path) {
		Ok(id) => id,
		Err(err) => {
			return Ok(HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(error_html_template("user", path, err.to_string())));
		}
	};
	check_slash(&mut path, user_id);
	let (params, errors) = parse_embed_parameters(&mut path, queries, &app.db).await;
	let link = format!("https://www.fimfiction.net/user/{path}");
	let body = match request_user(user_id, &app, params.refresh).await {
		Ok(user) => user_html_template(user, params, link, errors),
		Err(err) => error_html_template("user", path, err.to_string()),
	};
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

/// Requests a [User] from the cache. If it's not cached, it will be requested from Fimfiction.net (and also cached).
///
/// #### Errors
/// This function will return an error in the following cases:
/// - Can't connect to the database
/// - If the user is uncached:
///   - Can't connect to Fimfiction
///   - Can't deserialize response from Fimfiction
pub(crate) async fn request_user(
	id: i32, app: &AppState, recache: bool,
) -> Result<User, Box<dyn Error>> {
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

/// Formats a [User] to an HTML string for embedding.
///
/// #### Panics
///
/// Panics if stats are requested and the [User]'s stats can't be formatted.
pub(crate) fn user_html_template(
	user: User, parameters: Parameters, link: String, errors: Vec<String>,
) -> String {
	let mut errors = errors;
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(user.id))),
			Color::User => Some(user.color_hex),
			_ => unsupported_color(&mut errors, color.to_string(), user.color_hex),
		},
		None => Some(user.color_hex),
	};
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::None => None,
			Cover::User => map_picture(user.profile_pic_url),
			_ => unsupported_cover_opt(
				&mut errors,
				cover.to_string(),
				map_picture(user.profile_pic_url),
			),
		},
		None => map_picture(user.profile_pic_url),
	};
	let site_name = if parameters.stats {
		{
			let time = user.date_joined.format("%a %b %e %Y").to_string();
			format!(
				"Fimfiction - Joined: {time} 📅\nStories: {} 📚 Blogs: {} 📑 Followers: {} 👥",
				format_number_unit_metric(user.stories as f64, FormatType::MetricPrefix, 1, true)
					.unwrap(),
				format_number_unit_metric(user.blogs as f64, FormatType::MetricPrefix, 1, true)
					.unwrap(),
				format_number_unit_metric(user.followers as f64, FormatType::MetricPrefix, 1, true)
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
		errors: errors.to_vec(),
		user_name: None,
		user_link: None,
		html_comment: None,
		open_graph_type: String::from("profile"),
		open_graph_property: Some(String::from("profile:username")),
	};
	embed_html_template(data)
}
