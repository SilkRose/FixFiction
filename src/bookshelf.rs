use crate::database::{get_bookshelf, insert_bookshelf, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::bookshelf::BookshelfApi;
use crate::html_template::embed_html_template;
use crate::structs::{AppState, Bookshelf, Color, Cover, EmbedData, Parameters, User};
use crate::user::request_user;
use crate::utility::{
	get_color, map_picture, parse_fimfic_response, unsupported_color, unsupported_cover,
};
use crate::{check_recache, get_variant};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};

pub async fn request_bookshelf(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Bookshelf, Option<User>), Box<dyn std::error::Error>> {
	let bookshelf = get_bookshelf(id, &app.db).await?;
	let bookshelf = check_recache!(bookshelf, recache, app);
	match bookshelf {
		Some(bookshelf) => {
			let (bookshelf, user) = if let Some(user_id) = bookshelf.user_id {
				let user = request_user(user_id, app, recache).await?;
				(bookshelf, Some(user))
			} else {
				(bookshelf, None)
			};
			Ok((bookshelf, user))
		}
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/bookshelves/{id}?include=user");
			let api = parse_fimfic_response::<BookshelfApi<i32>>(&app.api, &fimfic).await?;
			if api.data.attributes.privacy == "private" {
				return Err("Fimfiction API Error: 404 – Resource not found".into());
			}
			let user = get_variant!(api.included, ApiIncluded::Author);
			if let Some(user) = user {
				let user = insert_user(None, user, &app.db).await?;
				let bookshelf =
					insert_bookshelf(Some(id), &api.data, Some(user.id), &app.db).await?;
				Ok((bookshelf, Some(user)))
			} else {
				let bookshelf = insert_bookshelf(Some(id), &api.data, None, &app.db).await?;
				Ok((bookshelf, None))
			}
		}
	}
}

pub fn bookshelf_html_template(
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
			format_number_unit_metric(bookshelf.stories as f64, FormatType::MetricPrefix, 1)
				.unwrap();
		let counts = match bookshelf.num_unread {
			Some(unread) => format!(
				"Stories: {stories} 📚 Unread Chapters: {} 📖",
				format_number_unit_metric(unread as f64, FormatType::MetricPrefix, 1).unwrap()
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
