use crate::database::{get_user, insert_user};
use crate::fimfiction_api::user::UserApi;
use crate::structs::{AppState, Color, Cover, Parameters, User};
use crate::utility::{map_picture, parse_fimfic_response};
use chrono::{TimeDelta, Utc};
use std::error::Error;
use url::form_urlencoded;

pub async fn request_user(id: i32, app: &AppState, recache: bool) -> Result<User, Box<dyn Error>> {
	let user = get_user(id, &app.db).await?;
	let user = match recache {
		true => user.filter(|user| {
			Utc::now()
				.checked_sub_signed(TimeDelta::seconds(app.cache_recache_age))
				.is_some_and(|max_age| user.date_cached >= max_age)
		}),
		false => user,
	};
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
	let mut text = String::new();
	text.push_str(r#"<!DOCTYPE html><html lang="en"><head>"#);
	text.push_str("<!-- FixFiction: https://github.com/SilkRose/FixFiction -->");
	text.push_str("<!-- Pinkie Pie is best pony! -->");
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			_ => Some(user.color_hex),
		},
		None => Some(user.color_hex),
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
		user.name
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{}" />"#,
		user.bio
	));
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::None => None,
			Cover::User => map_picture(user.profile_pic_url),
			Cover::Story => map_picture(user.profile_pic_url),
		},
		None => map_picture(user.profile_pic_url),
	};
	if let Some(cover) = cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
	text.push_str(&format!(r#"<meta property="og:url" content="{link}" />"#));
	text.push_str(r#"<meta property="og:type" content="profile" />"#);
	text.push_str(&format!(
		r#"<meta property="profile:username" content="{}" />"#,
		user.name
	));
	let mut site_name = if parameters.stats {
		{
			let time = user.date_joined.format("%a %b %e %Y").to_string();
			format!(
				"Fimfiction - Joined: {time} 📅\nStories: {} 📚 Blogs: {} 📑 Followers: {} 👥",
				user.stories, user.blogs, user.followers
			)
		}
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
	encode.append_pair("title", &user.name);
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	let encode = encode.finish();
	text.push_str(&format!(
			r#"<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed?{encode}" title="{}" />"#,
		user.name));
	text.push_str(r#"</head><body></body></html>"#);
	text
}
