use crate::structs::{
	AppState, Color, CompletionStatus, ContentRating, Cover, Parameters, Story, User,
};
use crate::user::{request_user, response_to_user};
use crate::utility::clean_content;
use chrono::{DateTime, TimeDelta, Utc};
use core::str;
use pony::fimfiction_api::story::StoryApi;
use pony::http::api_get_request;
use url::form_urlencoded;

pub async fn request_story(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Story, User), Box<dyn std::error::Error>> {
	let story = sqlx::query_as!(
		Story,
		r#"SELECT
			id, title, short_description, cover_medium_url,
			color_hex, views, words, chapters, comments,
			completion_status AS "completion_status: CompletionStatus",
			content_rating AS "content_rating: ContentRating",
			likes, dislikes, author_id, date_published, date_cached
		FROM Stories WHERE id = $1 LIMIT 1;"#,
		id
	)
	.fetch_optional(&app.db)
	.await?;

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
			let response = api_get_request(&app.api, &fimfic).await.unwrap();
			let api = response.json::<StoryApi<i32>>().await.unwrap();
			let author = api
				.included
				.iter()
				.find(|author| author.id == api.data.relationships.author.data.id)
				.unwrap();
			let user = response_to_user(&author.clone(), &app.db).await?;
			let story = sqlx::query_as!(
				Story,
				r#"INSERT INTO Stories (
					id, title, short_description, cover_medium_url,
					color_hex, views, words, chapters, comments,
					completion_status, content_rating,
					likes, dislikes, author_id, date_published)
				VALUES
					($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
				ON CONFLICT(id) DO UPDATE SET
					title = EXCLUDED.title,
					short_description = EXCLUDED.short_description,
					cover_medium_url = EXCLUDED.cover_medium_url,
					color_hex = EXCLUDED.color_hex,
					views = EXCLUDED.views,
					words = EXCLUDED.words,
					chapters = EXCLUDED.chapters,
					comments = EXCLUDED.comments,
					completion_status = EXCLUDED.completion_status,
					content_rating = EXCLUDED.content_rating,
					likes = EXCLUDED.likes,
					dislikes = EXCLUDED.dislikes,
					author_id = EXCLUDED.author_id,
					date_published = EXCLUDED.date_published,
					date_cached = now()
				RETURNING 
					id, title, short_description, cover_medium_url,
					color_hex, views, words, chapters, comments,
					completion_status AS "completion_status: CompletionStatus",
					content_rating AS "content_rating: ContentRating",
					likes, dislikes, author_id, date_published, date_cached;"#,
				id,
				clean_content(api.data.attributes.title),
				clean_content(api.data.attributes.short_description),
				api.data.attributes.cover_image.map(|cover| cover.medium),
				api.data.attributes.color.hex.trim_start_matches("#"),
				api.data.attributes.num_views,
				api.data.attributes.num_words,
				api.data.attributes.num_chapters,
				api.data.attributes.num_comments,
				CompletionStatus::from(api.data.attributes.completion_status) as _,
				ContentRating::from(api.data.attributes.content_rating) as _,
				api.data.attributes.num_likes,
				api.data.attributes.num_dislikes,
				user.id,
				DateTime::parse_from_rfc3339(
					&api.data
						.attributes
						.date_published
						.expect("All published stories should be published.")
				)?
			)
			.fetch_one(&app.db)
			.await?;
			Ok((story, user))
		}
	}
}

pub fn story_html_template(story: Story, user: User, parameters: Parameters, link: String) -> String {
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
	let site_name = if parameters.stats {
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
		&format!(
			"Fimfiction - Published: {time} 📅 Status: {status}\nRating: {rating} {likes_dislikes}Views: {} 📈\nComments: {} 💬 Chapters: {} 📖 Words: {} 📝",
			story.views, story.comments, story.chapters, story.words
		)
	} else {
		"Fimfiction"
	};
	text.push_str(&format!(
		r#"<meta property="og:site_name" content="{site_name}" />"#
	));
	text.push_str(r#"<meta property="twitter:site" content="fimfiction" />"#);
	text.push_str(r#"<meta property="twitter:card" content="summary" />"#);
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair("type", "rich");
	encode.append_pair("version", "1");
	encode.append_pair("provider_name", site_name);
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
