use crate::database::{get_story, insert_story};
use crate::structs::{
	AppState, Color, CompletionStatus, ContentRating, Cover, Parameters, Story, User,
};
use crate::user::{request_user, response_to_user};
use crate::utility::parse_fimfic_response;
use chrono::{TimeDelta, Utc};
use pony::fimfiction_api::story::StoryApi;
use url::form_urlencoded;

pub async fn request_story(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Story, User), Box<dyn std::error::Error>> {
	let story = get_story(id, &app.db).await?;
	let story = match recache {
		true => story.filter(|story| {
			Utc::now()
				.checked_sub_signed(TimeDelta::seconds(app.cache_recache_age))
				.is_some_and(|max_age| story.date_cached >= max_age)
		}),
		false => story,
	};
	match story {
		Some(story) => {
			let user = request_user(story.author_id, app, recache).await?;
			Ok((story, user))
		}
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/stories/{id}");
			let api = parse_fimfic_response::<StoryApi<i32>>(&app.api, &fimfic).await?;
			let author = api
				.included
				.iter()
				.find(|author| author.id == api.data.relationships.author.data.id)
				.ok_or("Fimfiction API error: no author included")?;
			let user = response_to_user(&author.clone(), &app.db).await?;
			let story = insert_story(id, api, user.id, &app.db).await?;
			Ok((story, user))
		}
	}
}

pub fn story_html_template(
	story: Story, user: User, parameters: Parameters, link: String, errors: String,
) -> String {
	let mut text = String::new();
	text.push_str(r#"<!DOCTYPE html><html lang="en"><head>"#);
	text.push_str("<!-- FixFiction: https://github.com/SilkRose/FixFiction -->");
	text.push_str("<!-- Pinkie Pie is best pony! -->");
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::User => Some(user.color_hex),
			Color::Story => Some(story.color_hex),
		},
		None => Some(story.color_hex),
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
		story.title
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{}" />"#,
		story.short_description
	));
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::Story => story.cover_medium_url,
			Cover::User => user.profile_pic_256,
			Cover::None => None,
		},
		None => story.cover_medium_url,
	};
	if let Some(cover) = cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
	text.push_str(&format!(r#"<meta property="og:url" content="{link}" />"#));
	text.push_str(r#"<meta property="og:type" content="book" />"#);
	text.push_str(&format!(
		r#"<meta property="book:author" content="{}" />"#,
		user.link
	));
	let mut site_name = if parameters.stats {
		let time = story.date_published.format("%a %b %e %Y").to_string();
		let status = match story.completion_status {
			CompletionStatus::Incomplete => "Incomplete 🔄",
			CompletionStatus::Complete => "Complete ✅",
			CompletionStatus::Hiatus => "Hiatus ⏸",
			CompletionStatus::Cancelled => "Cancelled ❌",
		};
		let rating = match story.content_rating {
			ContentRating::Everyone => "Everyone 🇪",
			ContentRating::Teen => "Teen 🇹",
			ContentRating::Mature => "Mature 🇲",
		};
		let likes_dislikes = if story.likes == -1 && story.dislikes == -1 {
			String::new()
		} else {
			format!("Likes: {} 👍 Dislikes: {} 👎 ", story.likes, story.dislikes)
		};
		format!(
			"Fimfiction - Published: {time} 📅 Status: {status}\nRating: {rating} {likes_dislikes}Views: {} 📈\nComments: {} 💬 Chapters: {} 📖 Words: {} 📝",
			story.views, story.comments, story.chapters, story.words
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
	encode.append_pair("title", &story.title);
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
