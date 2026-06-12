//! Request a [Bookshelf] and to format it in HTML.

use crate::database::{get_bookshelf, insert_bookshelf, insert_user};
use crate::error::{EmbedError, EmbedResult, Result};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::bookshelf::BookshelfApi;
use crate::html_template::{EmbedData, embed_html_template};
use crate::parameters::{Color, Cover, Parameters, parse_embed_parameters};
use crate::user::{User, request_user};
use crate::utility::{
	check_slash, get_color, map_picture, parse_fimfic_response, parse_id, unsupported_color,
	unsupported_cover,
};
use crate::{check_recache, get_variant};
use actix_web::web::{Path, Query, ThinData};
use actix_web::{HttpResponse, Responder, get};
use chrono::{DateTime, TimeDelta, Utc};
use pony::http::Request;
use pony::number_format::{FormatType, format_number_unit_metric};
use sqlx::{Pool, Postgres};
use std::collections::HashMap;

/// Fimfiction bookshelf data converted into a more usable structure
#[derive(Debug, Clone)]
pub(crate) struct Bookshelf {
	pub(crate) id: i32,
	pub(crate) name: String,
	pub(crate) description: String,
	pub(crate) link: String,
	pub(crate) color: String,
	pub(crate) icon_url: String,
	pub(crate) stories: i32,
	pub(crate) num_unread: Option<i32>,
	pub(crate) track_unread: bool,
	pub(crate) quick_add: bool,
	pub(crate) email_update: bool,
	pub(crate) user_id: Option<i32>,
	pub(crate) order_pos: i32,
	pub(crate) date_created: DateTime<Utc>,
	pub(crate) date_modified: DateTime<Utc>,
	pub(crate) date_cached: DateTime<Utc>,
}

/// The `bookshelf/` endpoint.
///
/// Requests a bookshelf by ID.
#[get("/bookshelf/{id:.*}")]
async fn get_bookshelf_endpoint(
	api: ThinData<Request>, db: ThinData<Pool<Postgres>>, path: Path<String>,
	queries: Query<HashMap<String, String>>,
) -> EmbedResult<impl Responder> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let bookshelf_id = parse_id(&path).map_embed_err("bookshelf", &path)?;
	check_slash(&mut path, bookshelf_id);
	let (params, errors) = parse_embed_parameters(&mut path, queries, &db).await;
	let link = format!("https://www.fimfiction.net/bookshelf/{path}");
	let (group, founder) = request_bookshelf(bookshelf_id, &api, &db, params.refresh)
		.await
		.map_embed_err("bookshelf", &path)?;
	let body = bookshelf_html_template(group, founder, params, link, errors);
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

/// Requests a [Bookshelf] from the cache. If it's not cached, it will be requested from Fimfiction.net (and also cached).
///
/// #### Errors
/// This function will return an error in the following cases:
/// - Can't connect to the database
/// - If the bookshelf is uncached:
///   - Can't connect to Fimfiction
///   - Can't deserialize response from Fimfiction
pub(crate) async fn request_bookshelf(
	id: i32, api: &Request, db: &Pool<Postgres>, recache: bool,
) -> Result<(Bookshelf, Option<User>)> {
	let bookshelf = get_bookshelf(id, db).await?;
	let bookshelf = check_recache!(bookshelf, recache, app);
	match bookshelf {
		Some(bookshelf) => {
			let (bookshelf, user) = if let Some(user_id) = bookshelf.user_id {
				let user = request_user(user_id, api, db, recache).await?;
				(bookshelf, Some(user))
			} else {
				(bookshelf, None)
			};
			Ok((bookshelf, user))
		}
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/bookshelves/{id}?include=user");
			let api = parse_fimfic_response::<BookshelfApi<i32>>(api, &fimfic).await?;
			if api.data.attributes.privacy == "private" {
				return Err("Fimfiction API Error: 4040 – Resource not found".into());
			}
			let user = get_variant!(api.included, ApiIncluded::Author);
			if let Some(user) = user {
				let user = insert_user(None, user, db).await?;
				let bookshelf = insert_bookshelf(Some(id), &api.data, Some(user.id), db).await?;
				Ok((bookshelf, Some(user)))
			} else {
				let bookshelf = insert_bookshelf(Some(id), &api.data, None, db).await?;
				Ok((bookshelf, None))
			}
		}
	}
}

/// Formats a [Bookshelf] to an HTML string for embedding. Most bookshelves are registered to a [User], but not all.
///
/// #### Panics
///
/// Panics if stats are requested and the [Bookshelf]'s number of stories or unread chapters can't be formatted.
pub(crate) fn bookshelf_html_template(
	bookshelf: Bookshelf, user: Option<User>, parameters: Parameters, link: String,
	errors: Vec<String>,
) -> String {
	let mut errors = errors;
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(bookshelf.id))),
			Color::User => match user {
				Some(ref user) => Some(user.color_hex.to_string()),
				None => unsupported_color(
					&mut errors,
					color.to_string(),
					get_color(Some(bookshelf.id)),
				),
			},
			_ => unsupported_color(&mut errors, color.to_string(), bookshelf.color),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::User => user
					.clone()
					.map(|user| user.color_hex)
					.or(Some(bookshelf.color)),
				Cover::None => None,
				_ => Some(bookshelf.color),
			},
			None => Some(bookshelf.color),
		},
	};
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::User => match user {
				Some(ref user) => map_picture(user.profile_pic_url.clone()),
				None => unsupported_cover(&mut errors, cover.to_string(), bookshelf.icon_url),
			},
			Cover::None => None,
			_ => unsupported_cover(&mut errors, cover.to_string(), bookshelf.icon_url),
		},
		None => Some(bookshelf.icon_url),
	};
	let site_name = if parameters.stats {
		let created = bookshelf.date_created.format("%a %b %e %Y").to_string();
		let modified = bookshelf.date_modified.format("%a %b %e %Y").to_string();
		let stats = match (
			bookshelf.track_unread,
			bookshelf.email_update,
			bookshelf.quick_add,
		) {
			(true, true, true) => "Track Unread: ✅ Email Updates: ✅ Quick Add: ✅",
			(true, true, false) => "Track Unread: ✅ Email Updates: ✅ Quick Add: 🚫",
			(true, false, true) => "Track Unread: ✅ Email Updates: 🚫 Quick Add: ✅",
			(true, false, false) => "Track Unread: ✅ Email Updates: 🚫 Quick Add: 🚫",
			(false, true, true) => "Track Unread: 🚫 Email Updates: ✅ Quick Add: ✅",
			(false, true, false) => "Track Unread: 🚫 Email Updates: ✅ Quick Add: 🚫",
			(false, false, true) => "Track Unread: 🚫 Email Updates: 🚫 Quick Add: ✅",
			(false, false, false) => "Track Unread: 🚫 Email Updates: 🚫 Quick Add: 🚫",
		};
		let stories =
			format_number_unit_metric(bookshelf.stories as f64, FormatType::MetricPrefix, 1, true)
				.unwrap();
		let counts = match bookshelf.num_unread {
			Some(unread) => format!(
				"Stories: {stories} 📚 Unread Chapters: {} 📖",
				format_number_unit_metric(unread as f64, FormatType::MetricPrefix, 1, true)
					.unwrap()
			),
			None => format!("Stories: {stories} 📚"),
		};
		format!("Fimfiction - Created: {created} 📅 Modified: {modified} 🕒\n{stats}\n{counts}")
	} else {
		"Fimfiction".to_string()
	};
	let data = EmbedData {
		title: bookshelf.name,
		description: bookshelf.description,
		link,
		color,
		cover,
		site_name,
		site_url: String::from("https://www.fimfiction.net/"),
		errors: errors.to_vec(),
		user_name: user.clone().map(|user| user.name),
		user_link: user.map(|user| user.link),
		html_comment: None,
		open_graph_type: String::from("profile"),
		open_graph_property: Some(String::from("profile:username")),
	};
	embed_html_template(data)
}
