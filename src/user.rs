//! Request a [User] and to format it in HTML.

use crate::check_recache;
use crate::database::Db;
use crate::error::{EmbedError, EmbedResult, Error, Result};
use crate::fimfiction_api::user::{UserApi, UserData};
use crate::html_template::{EmbedData, embed_html_template};
use crate::parameters::{Color, Cover, Parameters, parse_embed_parameters};
use crate::utility::{
	check_slash, get_color, map_picture, parse_fimfic_response, parse_id, unsupported_color,
	unsupported_cover_opt,
};
use actix_web::web::{Path, Query, ThinData};
use actix_web::{HttpResponse, Responder, get};
use chrono::{DateTime, TimeDelta, Utc};
use pony::http::Request;
use pony::number_format::{FormatType, format_number_unit_metric};
use std::collections::HashMap;

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

impl TryFrom<UserData<i32>> for User {
	type Error = Error;
	/// Converts Fimfiction's API response [UserData] into a [User]
	fn try_from(value: UserData<i32>) -> Result<Self> {
		let pfp_url = (!value.attributes.avatar.r64.ends_with("none_64.png")).then_some(
			value
				.attributes
				.avatar
				.r256
				.trim_end_matches("-256")
				.to_string(),
		);
		let hex = value.attributes.color.hex.trim_start_matches("#");
		let user = Self {
			id: value.id.parse::<i32>()?,
			name: value.attributes.name,
			bio: value.attributes.bio,
			link: value.meta.url,
			followers: value.attributes.num_followers,
			stories: value.attributes.num_stories,
			blogs: value.attributes.num_blog_posts,
			profile_pic_url: pfp_url,
			color_hex: hex.to_string(),
			date_joined: DateTime::parse_from_rfc3339(&value.attributes.date_joined)
				.map_err(|_| "FixFiction Error: failed to parse date joined")?
				.into(),
			date_cached: Utc::now(),
		};
		Ok(user)
	}
}

/// The `user/` endpoint.
///
/// Requests a user by ID.
#[get("/user/{id:.*}")]
async fn get_user_endpoint(
	api: ThinData<Request>, db: ThinData<Db>, path: Path<String>,
	queries: Query<HashMap<String, String>>,
) -> EmbedResult<impl Responder> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let user_id = parse_id(&path).map_embed_err("user", &path)?;
	check_slash(&mut path, user_id);
	let (params, errors) = parse_embed_parameters(&mut path, queries, &db).await;
	let link = format!("https://www.fimfiction.net/user/{path}");
	let user = request_user(user_id, &api, &db, params.refresh)
		.await
		.map_embed_err("user", &path)?;
	let body = user_html_template(user, params, link, errors);
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
pub(crate) async fn request_user(id: i32, api: &Request, db: &Db, recache: bool) -> Result<User> {
	let user = db.get_user(id).await?;
	let user = check_recache!(user, recache, app);
	match user {
		Some(user) => Ok(user),
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/users/{id}");
			let api = parse_fimfic_response::<UserApi<i32>>(api, &fimfic).await?;
			let user = User::try_from(api.data)?;
			db.insert_user(&user).await?;
			Ok(user)
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
