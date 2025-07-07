use crate::database::{get_blog, insert_blog, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::blog::BlogApi;
use crate::story::request_story;
use crate::structs::{AppState, Blog, Color, Cover, Parameters, Story, User};
use crate::user::request_user;
use crate::utility::{map_cover, map_picture, parse_fimfic_response};
use crate::{check_recache, get_variant};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};
use url::form_urlencoded;

pub async fn request_blog(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Blog, User, Option<Story>), Box<dyn std::error::Error>> {
	let blog = get_blog(id, &app.db).await?;
	let blog = check_recache!(blog, recache, app);
	match blog {
		Some(blog) => {
			let (story, user) = if let Some(story_id) = blog.story_id {
				let (story, user) = request_story(story_id, app, recache).await?;
				(Some(story), user)
			} else {
				(None, request_user(blog.author_id, app, recache).await?)
			};
			Ok((blog, user, story))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/blog-posts/{id}?include=author&fields[blog_post]=title,date_posted,content,num_views,num_comments,tagged_story"
			);
			let api = parse_fimfic_response::<BlogApi<i32>>(&app.api, &fimfic).await?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let story_id = (api.data.relationships.tagged_story.data.id != "0")
				.then_some(api.data.relationships.tagged_story.data.id.parse::<i32>()?);
			let (story, user) = if let Some(story_id) = story_id {
				let (story, user) = request_story(story_id, app, recache).await?;
				(Some(story), user)
			} else {
				(None, insert_user(None, author, &app.db).await?)
			};
			let blog = insert_blog(Some(id), &api.data, user.id, story_id, &app.db).await?;
			Ok((blog, user, story))
		}
	}
}

pub fn blog_html_template(
	blog: Blog, user: User, story: Option<Story>, parameters: Parameters, link: String,
	errors: String,
) -> String {
	let mut text = String::new();
	text.push_str(r#"<!DOCTYPE html><html lang="en"><head>"#);
	text.push_str("<!-- FixFiction: https://github.com/SilkRose/FixFiction -->");
	text.push_str("<!-- Pinkie Pie is best pony! -->");
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::Story => Some(
				story
					.clone()
					.map(|story| story.color_hex)
					.unwrap_or(user.color_hex),
			),
			Color::User => Some(user.color_hex),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => match story {
					Some(ref story) => Some(story.color_hex.clone()),
					None => Some(user.color_hex),
				},
				Cover::User => Some(user.color_hex),
				Cover::None => None,
			},
			None => Some(user.color_hex),
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
		blog.title
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{}" />"#,
		blog.content
	));
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::User => map_picture(user.profile_pic_url),
			Cover::Story => story
				.map(|story| map_cover(story.cover_url))
				.unwrap_or(map_picture(user.profile_pic_url)),
			Cover::None => None,
		},
		None => map_picture(user.profile_pic_url),
	};
	if let Some(cover) = cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
	text.push_str(&format!(r#"<meta property="og:url" content="{link}" />"#));
	text.push_str(r#"<meta property="og:type" content="article" />"#);
	text.push_str(&format!(
		r#"<meta property="article:author" content="{}" />"#,
		user.link
	));
	text.push_str(&format!(
		r#"<meta property="article:published_time" content="{}" />"#,
		blog.date_posted
	));
	let mut site_name = if parameters.stats {
		let time = blog.date_posted.format("%a %b %e %Y").to_string();
		format!(
			"Fimfiction - Posted: {time} 📅\nViews: {} 📈 Comments: {} 💬",
			format_number_unit_metric(blog.views as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(blog.comments as f64, FormatType::MetricPrefix, 1).unwrap(),
		)
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
	encode.append_pair("title", &blog.title);
	encode.append_pair("author_name", &user.name);
	encode.append_pair("author_url", &user.link);
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	let encode = encode.finish();
	text.push_str(&format!(
		r#"<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed?{encode}" title="{}" />"#,
		user.name));
	text.push_str(r#"</head><body></body></html>"#);
	text
}
