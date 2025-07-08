use crate::database::{get_chapter, get_story_chapter, insert_chapter, insert_story, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::chapter::ChapterApi;
use crate::fimfiction_api::story::StoryApi;
use crate::story::request_story;
use crate::structs::{
	AppState, Chapter, Color, CompletionStatus, ContentRating, Cover, Parameters, Story, User,
};
use crate::utility::{map_cover, map_picture, map_tags, parse_fimfic_response};
use crate::{check_recache, get_variant, get_variants};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};
use url::form_urlencoded;

pub async fn request_chapter(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Chapter, Story, User), Box<dyn std::error::Error>> {
	let chapter = get_chapter(id, &app.db).await?;
	let chapter = check_recache!(chapter, recache, app);
	match chapter {
		Some(chapter) => {
			let (story, user) = request_story(chapter.story_id, app, recache).await?;
			Ok((chapter, story, user))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/chapters/{id}?include=story,story.author"
			);
			let api = parse_fimfic_response::<ChapterApi<i32>>(&app.api, &fimfic).await?;
			let story = get_variant!(api.included, ApiIncluded::Story)
				.ok_or("Fimfiction API error: no story included")?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let tags = get_variants!(api.included, ApiIncluded::Tag).collect::<Vec<_>>();
			let tags = map_tags(tags);
			let author = insert_user(None, author, &app.db).await?;
			let story = insert_story(None, story.clone(), &tags, author.id, &app.db).await?;
			let chapter = insert_chapter(Some(id), api.data, story.id, &app.db).await?;
			Ok((chapter, story, author))
		}
	}
}

pub async fn request_story_chapters(
	story_id: i32, chapter_num: i32, app: &AppState, recache: bool,
) -> Result<(Chapter, Story, User), Box<dyn std::error::Error>> {
	let chapter = get_story_chapter(story_id, chapter_num, &app.db).await?;
	let chapter = check_recache!(chapter, recache, app);
	match chapter {
		Some(chapter) => {
			let (story, user) = request_story(chapter.story_id, app, recache).await?;
			Ok((chapter, story, user))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/stories/{story_id}?include=author,chapters,tags"
			);
			let api = parse_fimfic_response::<StoryApi<i32>>(&app.api, &fimfic).await?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let tags = get_variants!(api.included, ApiIncluded::Tag).collect::<Vec<_>>();
			let tags = map_tags(tags);
			let author = insert_user(None, author, &app.db).await?;
			let story = insert_story(None, api.data, &tags, author.id, &app.db).await?;
			let mut chapters = Vec::new();
			for chapter in &api.included {
				if let ApiIncluded::Chapter(data) = chapter {
					let inserted = insert_chapter(None, data.clone(), story_id, &app.db)
						.await
						.unwrap();
					chapters.push(inserted);
				}
			}
			let chapter = chapters
				.into_iter()
				.find(|chapter| chapter.chapter_num == chapter_num)
				.ok_or("FixFiction error: chapter not found")?;
			Ok((chapter, story, author))
		}
	}
}

pub fn chapter_html_template(
	chapter: Chapter, story: Story, user: User, parameters: Parameters, link: String,
	errors: String,
) -> String {
	let author = match parameters.tags {
		true => format!("{} – {}\nTags: {}", user.name, story.title, story.tags),
		false => format!("{} – {}", user.name, story.title),
	};
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
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => Some(story.color_hex),
				Cover::User => Some(user.color_hex),
				Cover::None => None,
			},
			None => Some(story.color_hex),
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
		chapter.title
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{}" />"#,
		story.short_description
	));
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::Story => map_cover(story.cover_url),
			Cover::User => map_picture(user.profile_pic_url),
			Cover::None => None,
		},
		None => map_cover(story.cover_url),
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
		let time = chapter.date_published.format("%a %b %e %Y").to_string();
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
			format!(
				"Likes: {} 👍 Dislikes: {} 👎 ",
				format_number_unit_metric(story.likes as f64, FormatType::MetricPrefix, 1).unwrap(),
				format_number_unit_metric(story.dislikes as f64, FormatType::MetricPrefix, 1)
					.unwrap()
			)
		};
		format!(
			"Fimfiction - Published: {time} 📅 Status: {status}\nRating: {rating} {likes_dislikes}Views: {}/{} 📈\nComments: {} 💬 Chapter: {}/{} 📖 Words: {}/{} 📝",
			format_number_unit_metric(chapter.views as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(story.views as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(story.comments as f64, FormatType::MetricPrefix, 1).unwrap(),
			chapter.chapter_num,
			story.chapters,
			format_number_unit_metric(chapter.words as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(story.words as f64, FormatType::MetricPrefix, 1).unwrap(),
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
	encode.append_pair("title", &chapter.title);
	encode.append_pair("author_name", &author);
	encode.append_pair("author_url", &user.link);
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	let encode = encode.finish();
	text.push_str(&format!(
		r#"<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed?{encode}" title="{author}" />"#));
	text.push_str(r#"</head><body></body></html>"#);
	text
}
