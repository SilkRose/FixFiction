//! Request a [Group] and to format it in HTML.

use crate::database::{get_group, insert_group, insert_user};
use crate::error::{Result, error_html_template};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::group::GroupApi;
use crate::html_template::{EmbedData, embed_html_template};
use crate::parameters::{Color, Cover, Parameters, parse_embed_parameters};
use crate::thread::{request_thread, thread_html_template};
use crate::user::{User, request_user};
use crate::utility::{
	check_slash, check_thread_slash, get_color, map_picture, parse_fimfic_response, parse_id,
	parse_thread_id, unsupported_color_opt, unsupported_cover_opt,
};
use crate::{check_recache, get_variant};
use actix_web::web::{Path, Query, ThinData};
use actix_web::{HttpResponse, Responder, get};
use chrono::{DateTime, TimeDelta, Utc};
use pony::http::Request;
use pony::number_format::{FormatType, format_number_unit_metric};
use sqlx::{Pool, Postgres};
use std::collections::HashMap;

/// Fimfiction group data converted into a more usable structure
#[derive(Debug, Clone)]
pub(crate) struct Group {
	pub(crate) id: i32,
	pub(crate) name: String,
	pub(crate) description: String,
	pub(crate) link: String,
	pub(crate) members: i32,
	pub(crate) stories: i32,
	pub(crate) founder_id: i32,
	pub(crate) nsfw: bool,
	pub(crate) open: bool,
	pub(crate) hidden: bool,
	pub(crate) icon_url: Option<String>,
	pub(crate) date_created: DateTime<Utc>,
	pub(crate) date_cached: DateTime<Utc>,
}

/// The `group/` endpoint.
///
/// Requests a group by ID.
#[get("/group/{id:.*}")]
async fn get_group_endpoint(
	api: ThinData<Request>, db: ThinData<Pool<Postgres>>, path: Path<String>,
	queries: Query<HashMap<String, String>>,
) -> Result<impl Responder> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let group_id = match parse_id(&path) {
		Ok(id) => id,
		Err(err) => {
			return Ok(HttpResponse::Ok()
				.content_type("text/html; charset=utf-8")
				.body(error_html_template("group", path, err.to_string())));
		}
	};
	let thread_id = parse_thread_id(&path);
	if let Some(thread_id) = thread_id {
		check_thread_slash(&mut path, thread_id);
	} else {
		check_slash(&mut path, group_id);
	}
	let (params, errors) = parse_embed_parameters(&mut path, queries, &db).await;
	let link = format!("https://www.fimfiction.net/group/{path}");

	let body = match thread_id {
		Some(thread_id) => {
			match request_thread(group_id, thread_id, &api, &db, params.refresh).await {
				Ok((group, founder, thread_data)) => match thread_data {
					Some(thread_data) => {
						thread_html_template(group, founder, thread_data, params, link, errors)
					}
					None => group_html_template(group, founder, params, link, errors),
				},
				Err(err) => error_html_template("group", path, err.to_string()),
			}
		}
		None => match request_group(group_id, &api, &db, params.refresh).await {
			Ok((group, founder)) => group_html_template(group, founder, params, link, errors),
			Err(err) => error_html_template("group", path, err.to_string()),
		},
	};

	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

/// Requests a [Group] from the cache. If it's not cached, it will be requested from Fimfiction.net (and also cached).
///
/// #### Errors
/// This function will return an error in the following cases:
/// - Can't connect to the database
/// - If the group is uncached:
///   - Can't connect to Fimfiction
///   - Can't deserialize response from Fimfiction
pub(crate) async fn request_group(
	id: i32, api: &Request, db: &Pool<Postgres>, recache: bool,
) -> Result<(Group, User)> {
	let group = get_group(id, db).await?;
	let group = check_recache!(group, recache, app);
	match group {
		Some(group) => {
			let founder = request_user(group.founder_id, api, db, recache).await?;
			Ok((group, founder))
		}
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/groups/{id}?include=founder");
			let api = parse_fimfic_response::<GroupApi<i32>>(api, &fimfic).await?;
			let founder = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no founder included")?;
			let founder = insert_user(None, founder, db).await?;
			let group = insert_group(Some(id), &api.data, db).await?;
			Ok((group, founder))
		}
	}
}

/// Formats a [Group] to an HTML string for embedding. All groups have a founding [User].
///
/// #### Panics
///
/// Panics if stats are requested and the [Group]'s stats can't be formatted.
pub(crate) fn group_html_template(
	group: Group, founder: User, parameters: Parameters, link: String, errors: Vec<String>,
) -> String {
	let mut errors = errors;
	let founder_name = match parameters.stats {
		true => Some(format!("Founder: {}", founder.name)),
		false => None,
	};
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::Founder => Some(founder.color_hex),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(group.id))),
			_ => unsupported_color_opt(&mut errors, color.to_string(), Some(founder.color_hex)),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => Some(get_color(Some(group.id))),
				Cover::User | Cover::Founder => Some(founder.color_hex),
				Cover::None => None,
			},
			None => Some(get_color(Some(group.id))),
		},
	};
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::Founder => map_picture(founder.profile_pic_url),
			Cover::None => None,
			_ => unsupported_cover_opt(
				&mut errors,
				cover.to_string(),
				map_picture(group.icon_url).or(map_picture(founder.profile_pic_url)),
			),
		},
		None => map_picture(group.icon_url).or(map_picture(founder.profile_pic_url)),
	};
	let site_name = if parameters.stats {
		let time = group.date_created.format("%a %b %e %Y").to_string();
		let mature = match group.nsfw {
			true => "Not safe for work: 🔞",
			false => "Safe for work: ✅",
		};
		let open = match group.open {
			true => "Open submissions: ✅",
			false => "Closed submissions: 🚫",
		};
		format!(
			"Fimfiction - Created: {time} 📅\nMembers: {} 👥 Stories: {} 📚\n{open} {mature}",
			format_number_unit_metric(group.members as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(group.stories as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
		)
	} else {
		"Fimfiction".to_string()
	};
	let data = EmbedData {
		title: group.name,
		description: group.description,
		link,
		color,
		cover,
		site_name,
		site_url: String::from("https://www.fimfiction.net/"),
		errors: errors.to_vec(),
		user_name: founder_name,
		user_link: Some(founder.link),
		html_comment: None,
		open_graph_type: String::from("profile"),
		open_graph_property: Some(String::from("profile:username")),
	};
	embed_html_template(data)
}
