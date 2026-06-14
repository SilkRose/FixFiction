//! Request a [Story] and to format it in HTML.

use crate::chapter::{chapter_html_template, request_story_chapters};
use crate::database::{
	get_story, get_tag, get_tag_links, insert_story, insert_tag, insert_tag_link, insert_user,
	remove_tag_links,
};
use crate::error::{EmbedError, EmbedResult, Result};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::story::StoryApi;
use crate::html_template::{EmbedData, embed_html_template};
use crate::parameters::{Color, Cover, Parameters, parse_embed_parameters};
use crate::tag::Tag;
use crate::user::{User, request_user};
use crate::utility::{
	get_color, map_cover, map_picture, map_tags, parse_chapter_number, parse_fimfic_response,
	parse_id, unsupported_color, unsupported_cover_opt,
};
use crate::{check_recache, get_variant, get_variants};
use actix_web::web::{Path, Query, ThinData};
use actix_web::{HttpResponse, Responder, get};
use chrono::{DateTime, TimeDelta, Utc};
use pony::http::Request;
use pony::number_format::{FormatType, format_number_unit_metric};
use serde::{Deserialize, Serialize};
use sqlx::prelude::Type;
use sqlx::{Pool, Postgres};
use std::collections::HashMap;

/// Fimfiction story data converted into a more usable structure
#[derive(Debug, Clone)]
pub(crate) struct Story {
	pub(crate) id: i32,
	pub(crate) title: String,
	pub(crate) short_description: String,
	pub(crate) description: String,
	pub(crate) published: bool,
	pub(crate) link: String,
	pub(crate) cover_url: Option<String>,
	pub(crate) color_hex: String,
	pub(crate) views: i32,
	pub(crate) total_views: i32,
	pub(crate) words: i32,
	pub(crate) chapters: i32,
	pub(crate) comments: i32,
	pub(crate) rating: i32,
	pub(crate) completion_status: CompletionStatus,
	pub(crate) content_rating: ContentRating,
	pub(crate) likes: i32,
	pub(crate) dislikes: i32,
	pub(crate) author_id: i32,
	pub(crate) date_modified: DateTime<Utc>,
	pub(crate) date_updated: DateTime<Utc>,
	pub(crate) date_published: DateTime<Utc>,
	pub(crate) date_cached: DateTime<Utc>,
}

/// Fimfiction story content rating data converted into a more usable structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "content_rating", rename_all = "lowercase")]
pub(crate) enum ContentRating {
	Everyone,
	Teen,
	Mature,
}

impl From<String> for ContentRating {
	/// Converts a Fimfiction API response string for story rating into [ContentRating]
	///
	/// #### Panics
	///
	/// Panics if Fimfiction returns a value not present.
	fn from(value: String) -> Self {
		match value.as_str() {
			"everyone" => ContentRating::Everyone,
			"teen" => ContentRating::Teen,
			"mature" => ContentRating::Mature,
			_ => unreachable!(), // This should never happen, but still want to add something here later.
		}
	}
}

/// Fimfiction story completion status data converted into a more usable structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "completion_status", rename_all = "lowercase")]
pub(crate) enum CompletionStatus {
	Incomplete,
	Complete,
	Hiatus,
	Cancelled,
}

impl From<String> for CompletionStatus {
	/// Converts a Fimfiction API response string for story status into [CompletionStatus]
	///
	/// #### Panics
	///
	/// Panics if Fimfiction returns a value not present.
	fn from(value: String) -> Self {
		match value.as_str() {
			"incomplete" => CompletionStatus::Incomplete,
			"complete" => CompletionStatus::Complete,
			"hiatus" => CompletionStatus::Hiatus,
			"cancelled" => CompletionStatus::Cancelled,
			_ => unreachable!(), // This should never happen, but still want to add something here later.
		}
	}
}

/// The `story/` endpoint.
///
/// Requests a story by ID.
/// May also include an ordinal chapter number.
#[get("/story/{id:.*}")]
async fn get_story_endpoint(
	api: ThinData<Request>, db: ThinData<Pool<Postgres>>, path: Path<String>,
	queries: Query<HashMap<String, String>>,
) -> EmbedResult<impl Responder> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let story_id = parse_id(&path).map_embed_err("story", &path)?;
	let chapter_num = parse_chapter_number(&path);
	let (params, errors) = parse_embed_parameters(&mut path, queries, &db).await;
	let link = format!("https://www.fimfiction.net/story/{path}");
	let body = if let Some(chapter_num) = chapter_num {
		let (chapter, story, user, tags) =
			request_story_chapters(story_id, chapter_num, &api, &db, params.refresh)
				.await
				.map_embed_err("story", &path)?;
		chapter_html_template(chapter, story, user, tags, params, link, errors)
	} else {
		let (story, user, tags) = request_story(story_id, &api, &db, params.refresh)
			.await
			.map_embed_err("story", &path)?;
		story_html_template(story, user, tags, params, link, errors)
	};
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

/// Requests a [Story] from the cache. If it's not cached, it will be requested from Fimfiction.net (and also cached).
///
/// #### Errors
/// This function will return an error in the following cases:
/// - Can't connect to the database
/// - If the story is uncached:
///   - Can't connect to Fimfiction
///   - Can't deserialize response from Fimfiction
pub(crate) async fn request_story(
	id: i32, api: &Request, db: &Pool<Postgres>, recache: bool,
) -> Result<(Story, User, Vec<Tag>)> {
	let story = get_story(id, db).await?;
	let story = check_recache!(story, recache, app);
	match story {
		Some(story) => {
			let user = request_user(story.author_id, api, db, recache).await?;
			let tag_links = get_tag_links(story.id, db).await?;
			let mut tags = Vec::with_capacity(tag_links.len());
			for link in tag_links {
				let tag = get_tag(link.tag_id, db)
					.await?
					.expect("Database constraint means this will never fail.");
				tags.push(tag);
			}
			Ok((story, user, tags))
		}
		None => {
			let fimfic =
				format!("https://www.fimfiction.net/api/v2/stories/{id}?include=author,tags");
			let api = parse_fimfic_response::<StoryApi<i32>>(api, &fimfic).await?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let api_tags = get_variants!(api.included, ApiIncluded::Tag).collect::<Vec<_>>();
			let user = User::try_from(author.clone())?;
			insert_user(&user, db).await?;
			let story = insert_story(Some(id), api.data, user.id, db).await?;
			remove_tag_links(id, db).await?;
			let mut tags = Vec::with_capacity(api_tags.len());
			for tag in api_tags {
				let tag = insert_tag(None, tag.clone(), db).await?;
				insert_tag_link(id, tag.id, db).await?;
				tags.push(tag);
			}
			Ok((story, user, tags))
		}
	}
}

/// Formats a [Story] to an HTML string for embedding. All stories are authored by a [User].
///
/// #### Panics
///
/// Panics if stats are requested and the [Story]'s stats can't be formatted.
pub(crate) fn story_html_template(
	story: Story, user: User, mut tags: Vec<Tag>, parameters: Parameters, link: String,
	errors: Vec<String>,
) -> String {
	let mut errors = errors;
	let mut text = String::new();
	let author = if parameters.tags {
		tags.sort();
		format!("{}\nTags: {}", user.name, map_tags(&tags))
	} else {
		user.name
	};
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::User => Some(user.color_hex),
			Color::Story => Some(story.color_hex),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(story.id))),
			_ => unsupported_color(&mut errors, color.to_string(), story.color_hex),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => Some(story.color_hex),
				Cover::User => Some(user.color_hex),
				Cover::None => None,
				_ => Some(user.color_hex),
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
	if let Some(ref cover) = cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
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
			format!(
				"Likes: {} 👍 Dislikes: {} 👎 ",
				format_number_unit_metric(story.likes as f64, FormatType::MetricPrefix, 1, true)
					.unwrap(),
				format_number_unit_metric(story.dislikes as f64, FormatType::MetricPrefix, 1, true)
					.unwrap()
			)
		};
		format!(
			"Fimfiction - Published: {time} 📅 Status: {status}\nRating: {rating} {likes_dislikes}Views: {} 📈\nComments: {} 💬 Chapters: {} 📖 Words: {} 📝",
			format_number_unit_metric(story.views as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(story.comments as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(story.chapters as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(story.words as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
		)
	} else {
		"Fimfiction".to_string()
	};
	let data = EmbedData {
		title: story.title,
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
