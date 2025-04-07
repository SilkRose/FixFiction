use crate::structs::{AppState, Color, Cover, Parameters, User};
use crate::utility::{clean_content, parse_fimfic_response};
use chrono::{DateTime, TimeDelta, Utc};
use core::str;
use pony::fimfiction_api::user::{UserApi, UserData};
use sqlx::{Pool, Postgres};
use std::error::Error;
use url::form_urlencoded;

pub async fn request_user(
	id: i32, app: &AppState, recache: bool,
) -> Result<User, Box<dyn std::error::Error>> {
	let user = sqlx::query_as!(
		User,
		"SELECT
			id, name, bio, link, followers,
			stories, blogs, profile_pic_256,
			color_hex, date_joined, date_cached
		FROM Authors WHERE id = $1 LIMIT 1;",
		id
	)
	.fetch_optional(&app.db)
	.await
	.map_err(|_| "FixFiction Error: database retrieval error")?;

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
			response_to_user(&api.data, &app.db).await
		}
	}
}

pub async fn response_to_user(
	data: &UserData<i32>, db: &Pool<Postgres>,
) -> Result<User, Box<dyn Error>> {
	let image = (!data.attributes.avatar.r64.ends_with("none_64.png"))
		.then_some(data.attributes.avatar.r256.clone());
	let user = sqlx::query_as!(
		User,
		"INSERT INTO Authors 
			(id, name, bio, link, followers, stories,
			blogs, profile_pic_256, color_hex, date_joined)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
		ON CONFLICT(id) DO UPDATE SET
			name = EXCLUDED.name,
			bio = EXCLUDED.bio,
			link = EXCLUDED.link,
			followers = EXCLUDED.followers,
			stories = EXCLUDED.stories,
			blogs = EXCLUDED.blogs,
			profile_pic_256 = EXCLUDED.profile_pic_256,
			color_hex = EXCLUDED.color_hex,
			date_joined = EXCLUDED.date_joined,
			date_cached = now()
		RETURNING
			id, name, bio, link, followers,
			stories, blogs, profile_pic_256,
			color_hex, date_joined, date_cached;",
		data.id.parse::<i32>()?,
		clean_content(data.attributes.name.clone()),
		clean_content(data.attributes.bio.clone()),
		data.meta.url,
		data.attributes.num_followers,
		data.attributes.num_stories,
		data.attributes.num_blog_posts,
		image,
		data.attributes.color.hex.trim_start_matches("#"),
		DateTime::parse_from_rfc3339(&data.attributes.date_joined)
			.map_err(|_| "FixFiction Error: failed to parse date joined")?
	)
	.fetch_one(db)
	.await
	.map_err(|_| "FixFiction Error: database insertion error")?;
	Ok(user)
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
			Cover::User => user.profile_pic_256,
			Cover::Story => user.profile_pic_256,
		},
		None => user.profile_pic_256,
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
