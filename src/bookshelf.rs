use crate::database::{get_bookshelf, insert_bookshelf, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::bookshelf::BookshelfApi;
use crate::structs::{AppState, Bookshelf, Color, Cover, Parameters, User};
use crate::user::request_user;
use crate::utility::{get_color, map_picture, parse_fimfic_response};
use crate::{check_recache, get_variant};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};
use url::form_urlencoded;

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
	bookshelf: Bookshelf, user: Option<User>, parameters: Parameters, link: String, errors: String,
) -> String {
	let mut text = String::new();
	text.push_str(r#"<!DOCTYPE html><html lang="en"><head>"#);
	text.push_str("<!-- FixFiction: https://github.com/SilkRose/FixFiction -->");
	text.push_str("<!-- Pinkie Pie is best pony! -->");
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::User => user.clone().map(|user| user.color_hex),
			Color::Random => Some(get_color(None)),
			Color::Modulo | Color::Story => Some(get_color(Some(bookshelf.id))),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => Some(get_color(Some(bookshelf.id))),
				Cover::User => user.clone().map(|user| user.color_hex),
				Cover::None => None,
			},
			None => Some(bookshelf.color),
		},
	};
	if let Some(color) = color {
		text.push_str(&format!(
			r##"<meta name="theme-color" content="#{color}" />"##
		));
	}
	text.push_str(&format!(r#"<link rel="canonical" href="{link}" />"#));
	text.push_str(&format!(
		r#"<meta http-equiv="refresh" content="0;url={link}" />"#
	));
	text.push_str(&format!(
		r#"<meta property="og:title" content="{}" />"#,
		bookshelf.name
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{}" />"#,
		bookshelf.description
	));
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::User | Cover::Story => user
				.clone()
				.map(|user| map_picture(user.profile_pic_url).unwrap()),
			Cover::None => None,
		},
		None => Some(bookshelf.icon_url),
	};
	if let Some(cover) = cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
	text.push_str(&format!(r#"<meta property="og:url" content="{link}" />"#));
	text.push_str(r#"<meta property="og:type" content="profile" />"#);
	if let Some(ref user) = user {
		text.push_str(&format!(
			r#"<meta property="profile:username" content="{}" />"#,
			user.link
		));
	}
	let mut site_name = if parameters.stats {
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
	if !errors.is_empty() {
		site_name = format!("{site_name}\n{errors}");
	}
	text.push_str(&format!(
		r#"<meta property="og:site_name" content="{site_name}" />"#
	));
	text.push_str(r#"<meta property="twitter:site" content="fimfiction" />"#);
	text.push_str(r#"<meta property="twitter:card" content="summary" />"#);
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair("type", "rich");
	encode.append_pair("version", "1");
	encode.append_pair("provider_name", &site_name);
	encode.append_pair("provider_url", "https://www.fimfiction.net/");
	encode.append_pair("title", &bookshelf.name);
	if let Some(user) = user {
		encode.append_pair("author_name", &user.name);
		encode.append_pair("author_url", &user.link);
	}
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	let encode = encode.finish();
	text.push_str(&format!(
		r#"<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed?{encode}" title="{}" />"#,
		&bookshelf.name));
	text.push_str(r#"</head><body></body></html>"#);
	text
}
