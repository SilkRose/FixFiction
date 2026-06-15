//! Request a [Chapter], or a chapter of a [Story], and to format it in HTML.

use crate::database::{
	get_chapter, get_story_chapter, insert_chapter, insert_story, insert_tag, insert_tag_link,
	insert_user, remove_tag_links,
};
use crate::error::{EmbedError, EmbedResult, Error, Result};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::chapter::{ChapterApi, ChapterData};
use crate::fimfiction_api::story::StoryApi;
use crate::html_template::{EmbedData, embed_html_template};
use crate::parameters::{Color, Cover, Parameters, parse_embed_parameters};
use crate::story::{CompletionStatus, ContentRating, Story, request_story};
use crate::tag::Tag;
use crate::user::User;
use crate::utility::{
	get_color, map_cover, map_picture, map_tags, parse_fimfic_response, parse_id,
	unsupported_color, unsupported_cover_opt,
};
use crate::{check_recache, get_variant, get_variants};
use actix_web::web::{Path, Query, ThinData};
use actix_web::{HttpResponse, Responder, get};
use chrono::{DateTime, TimeDelta, Utc};
use pony::http::Request;
use pony::number_format::{FormatType, format_number_unit_metric};
use sqlx::{Pool, Postgres};
use std::collections::HashMap;

/// Fimfiction chapter data converted into a more usable structure
#[derive(Debug, Clone)]
pub(crate) struct Chapter {
	pub(crate) id: i32,
	pub(crate) story_id: i32,
	pub(crate) chapter_num: i32,
	pub(crate) title: String,
	pub(crate) link: String,
	pub(crate) views: i32,
	pub(crate) words: i32,
	pub(crate) date_published: DateTime<Utc>,
	pub(crate) date_modified: DateTime<Utc>,
	pub(crate) date_cached: DateTime<Utc>,
}

impl TryFrom<ChapterData<i32>> for Chapter {
	type Error = Error;
	fn try_from(value: ChapterData<i32>) -> Result<Self> {
		let chapter = Self {
			id: value.id.parse()?,
			story_id: value.relationships.story.data.id.parse()?,
			chapter_num: value.attributes.chapter_number,
			title: value.attributes.title,
			link: value.meta.url,
			views: value.attributes.num_views,
			words: value.attributes.num_words,
			date_published: DateTime::parse_from_rfc3339(&value.attributes.date_published)
				.map_err(|_| "FixFiction Error: failed to parse date published")?
				.into(),
			date_modified: DateTime::parse_from_rfc3339(&value.attributes.date_modified)
				.map_err(|_| "FixFiction Error: failed to parse date modified")?
				.into(),
			date_cached: Utc::now(),
		};
		Ok(chapter)
	}
}

/// The `chapter/` endpoint.
///
/// Requests a chapter by ID.
/// More direct than `story/{id}/chapter/{num}`.
#[get("/chapter/{id:.*}")]
async fn get_chapter_endpoint(
	api: ThinData<Request>, db: ThinData<Pool<Postgres>>, path: Path<String>,
	queries: Query<HashMap<String, String>>,
) -> EmbedResult<impl Responder> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let chapter_id = parse_id(&path).map_embed_err("chapter", &path)?;
	let (params, errors) = parse_embed_parameters(&mut path, queries, &db).await;
	let link = format!("https://www.fimfiction.net/chapter/{path}");
	let (chapter, story, user, tags) = request_chapter(chapter_id, &api, &db, params.refresh)
		.await
		.map_embed_err("chapter", &path)?;
	let body = chapter_html_template(chapter, story, user, tags, params, link, errors);
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

/// Requests a [Chapter] from the cache. If it's not cached, it will be requested from Fimfiction.net (and also cached).
///
/// #### Errors
/// This function will return an error in the following cases:
/// - Can't connect to the database
/// - If the chapter is uncached:
///   - Can't connect to Fimfiction
///   - Can't deserialize response from Fimfiction
pub(crate) async fn request_chapter(
	id: i32, api: &Request, db: &Pool<Postgres>, recache: bool,
) -> Result<(Chapter, Story, User, Vec<Tag>)> {
	let chapter = get_chapter(id, db).await?;
	let chapter = check_recache!(chapter, recache, app);
	match chapter {
		Some(chapter) => {
			let (story, user, tags) = request_story(chapter.story_id, api, db, recache).await?;
			Ok((chapter, story, user, tags))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/chapters/{id}?include=story,story.author,story.tags"
			);
			let api = parse_fimfic_response::<ChapterApi<i32>>(api, &fimfic).await?;
			let story = get_variant!(api.included, ApiIncluded::Story)
				.ok_or("Fimfiction API error: no story included")?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let api_tags = get_variants!(api.included, ApiIncluded::Tag).collect::<Vec<_>>();
			let user = User::try_from(author.clone())?;
			insert_user(&user, db).await?;
			let story = insert_story(None, story.clone(), user.id, db).await?;
			remove_tag_links(story.id, db).await?;
			let mut tags = Vec::with_capacity(api_tags.len());
			for tag in api_tags {
				let tag = insert_tag(None, tag.clone(), db).await?;
				insert_tag_link(story.id, tag.id, db).await?;
				tags.push(tag);
			}
			let chapter = Chapter::try_from(api.data)?;
			insert_chapter(&chapter, db).await?;
			Ok((chapter, story, user, tags))
		}
	}
}

/// Requests a [Chapter], indexed ordinally from the parent [Story], from the cache. If it's not cached, it will be requested from Fimfiction.net (and also cached).
///
/// #### Errors
/// This function will return an error in the following cases:
/// - Can't connect to the database
/// - If the chapter is uncached:
///   - Can't connect to Fimfiction
///   - Can't deserialize response from Fimfiction
pub(crate) async fn request_story_chapters(
	story_id: i32, chapter_num: i32, api: &Request, db: &Pool<Postgres>, recache: bool,
) -> Result<(Chapter, Story, User, Vec<Tag>)> {
	let chapter = get_story_chapter(story_id, chapter_num, db).await?;
	let chapter = check_recache!(chapter, recache, app);
	match chapter {
		Some(chapter) => {
			let (story, user, tags) = request_story(chapter.story_id, api, db, recache).await?;
			Ok((chapter, story, user, tags))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/stories/{story_id}?include=author,chapters,tags"
			);
			let api = parse_fimfic_response::<StoryApi<i32>>(api, &fimfic).await?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let user = User::try_from(author.clone())?;
			insert_user(&user, db).await?;
			let story = insert_story(None, api.data, user.id, db).await?;
			let api_tags = get_variants!(api.included, ApiIncluded::Tag).collect::<Vec<_>>();
			remove_tag_links(story_id, db).await?;
			let mut tags = Vec::with_capacity(api_tags.len());
			for tag in api_tags {
				let tag = insert_tag(None, tag.clone(), db).await?;
				insert_tag_link(story_id, tag.id, db).await?;
				tags.push(tag);
			}
			let mut chapters = Vec::new();
			for chapter in api.included {
				if let ApiIncluded::Chapter(data) = chapter {
					let chapter = Chapter::try_from(data)?;
					insert_chapter(&chapter, db).await?;
					chapters.push(chapter);
				}
			}
			let chapter = chapters
				.into_iter()
				.find(|chapter| chapter.chapter_num == chapter_num)
				.ok_or("FixFiction error: chapter not found")?;
			Ok((chapter, story, user, tags))
		}
	}
}

/// Formats a [Chapter] to an HTML string for embedding. All chapters are a child of a [Story], which is authored by a [User].
///
/// #### Panics
///
/// Panics if stats are requested and the [Chapter]'s parent story's stats can't be formatted.
pub(crate) fn chapter_html_template(
	chapter: Chapter, story: Story, user: User, mut tags: Vec<Tag>, parameters: Parameters,
	link: String, errors: Vec<String>,
) -> String {
	let mut errors = errors;
	let author = if parameters.tags {
		tags.sort();
		format!("{} – {}\nTags: {}", user.name, story.title, map_tags(&tags))
	} else {
		format!("{} – {}", user.name, story.title)
	};
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::User => Some(user.color_hex),
			Color::Story => Some(story.color_hex),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(chapter.id))),
			_ => unsupported_color(&mut errors, color.to_string(), story.color_hex),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => Some(story.color_hex),
				Cover::User | Cover::Founder => Some(user.color_hex),
				Cover::None => None,
			},
			None => Some(story.color_hex),
		},
	};
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::Story => map_cover(story.cover_url),
			Cover::User => map_picture(user.profile_pic_url),
			Cover::None => None,
			_ => unsupported_cover_opt(&mut errors, cover.to_string(), map_cover(story.cover_url)),
		},
		None => map_cover(story.cover_url),
	};
	let site_name = if parameters.stats {
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
				format_number_unit_metric(story.likes as f64, FormatType::MetricPrefix, 1, true)
					.unwrap(),
				format_number_unit_metric(story.dislikes as f64, FormatType::MetricPrefix, 1, true)
					.unwrap()
			)
		};
		format!(
			"Fimfiction - Published: {time} 📅 Status: {status}\nRating: {rating} {likes_dislikes}Views: {}/{} 📈\nComments: {} 💬 Chapter: {}/{} 📖 Words: {}/{} 📝",
			format_number_unit_metric(chapter.views as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(story.views as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(story.comments as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			chapter.chapter_num,
			story.chapters,
			format_number_unit_metric(chapter.words as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(story.words as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
		)
	} else {
		"Fimfiction".to_string()
	};
	let data = EmbedData {
		title: chapter.title,
		description: story.short_description,
		link,
		color,
		cover,
		site_name,
		site_url: String::from("https://www.fimfiction.net/"),
		errors: errors.to_vec(),
		user_name: Some(author),
		user_link: Some(user.link),
		html_comment: None,
		open_graph_type: String::from("book"),
		open_graph_property: Some(String::from("book:author")),
	};
	embed_html_template(data)
}
