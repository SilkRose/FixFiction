//! Request a [Story] and to format it in HTML.

use crate::chapter::{chapter_html_template, request_story_chapters};
use crate::database::Db;
use crate::error::{EmbedError, EmbedResult, Error, Result};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::story::{StoryApi, StoryData};
use crate::html_template::{EmbedData, embed_html_template};
use crate::parameters::{Color, Cover, Parameters, parse_embed_parameters};
use crate::tag::Tag;
use crate::user::{User, request_user};
use crate::utility::{
	get_color, map_cover, map_picture, map_tags, parse_chapter_number, parse_date,
	parse_fimfic_response, parse_id, unsupported_color, unsupported_cover_opt,
};
use crate::{check_recache, get_variant, get_variants};
use actix_web::web::{Path, Query, ThinData};
use actix_web::{HttpResponse, Responder, get};
use chrono::{DateTime, TimeDelta, Utc};
use pony::http::Request;
use pony::number_format::{FormatType, format_number_unit_metric};
use serde::{Deserialize, Serialize};
use sqlx::prelude::Type;
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

impl TryFrom<StoryData<i32>> for Story {
	type Error = Error;
	/// Converts Fimfiction's API response [StoryData] into a [Story]
	fn try_from(value: StoryData<i32>) -> Result<Self> {
		let story = Story {
			id: value.id.parse()?,
			title: value.attributes.title,
			short_description: value.attributes.short_description,
			description: value.attributes.description,
			published: value.attributes.published,
			link: value.meta.url,
			cover_url: value
				.attributes
				.cover_image
				.map(|cover| cover.medium.trim_end_matches("-medium").to_string()),
			color_hex: value
				.attributes
				.color
				.hex
				.trim_start_matches("#")
				.to_string(),
			views: value.attributes.num_views,
			total_views: value.attributes.total_num_views,
			words: value.attributes.num_words,
			chapters: value.attributes.num_chapters,
			comments: value.attributes.num_comments,
			rating: value.attributes.rating,
			completion_status: CompletionStatus::try_from(value.attributes.completion_status)?,
			content_rating: ContentRating::try_from(value.attributes.content_rating)?,
			likes: value.attributes.num_likes,
			dislikes: value.attributes.num_dislikes,
			author_id: value.relationships.author.data.id.parse()?,
			date_modified: parse_date(value.attributes.date_modified, "modified")?.into(),
			date_updated: parse_date(
				value
					.attributes
					.date_updated
					.ok_or("Fimfictiion API error: no updated date")?,
				"updated",
			)?
			.into(),
			date_published: parse_date(
				value
					.attributes
					.date_published
					.ok_or("Fimfictiion API error: no publish date")?,
				"published",
			)?
			.into(),
			date_cached: Utc::now(),
		};
		Ok(story)
	}
}

/// Fimfiction story content rating data converted into a more usable structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "content_rating", rename_all = "lowercase")]
pub(crate) enum ContentRating {
	Everyone,
	Teen,
	Mature,
}

impl TryFrom<String> for ContentRating {
	type Error = Error;
	/// Converts a Fimfiction API response string for story rating into [ContentRating]
	fn try_from(value: String) -> Result<Self> {
		match value.as_str() {
			"everyone" => Ok(ContentRating::Everyone),
			"teen" => Ok(ContentRating::Teen),
			"mature" => Ok(ContentRating::Mature),
			_ => Err(format!(
				"FixFiction error: failed to parse string into content rating: {value}"
			)
			.into()),
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

impl TryFrom<String> for CompletionStatus {
	type Error = Error;
	/// Converts a Fimfiction API response string for story status into [CompletionStatus]
	fn try_from(value: String) -> Result<Self> {
		match value.as_str() {
			"incomplete" => Ok(CompletionStatus::Incomplete),
			"complete" => Ok(CompletionStatus::Complete),
			"hiatus" => Ok(CompletionStatus::Hiatus),
			"cancelled" => Ok(CompletionStatus::Cancelled),
			_ => Err(format!(
				"FixFiction error: failed to parse string into completion status: {value}"
			)
			.into()),
		}
	}
}

/// The `story/` endpoint.
///
/// Requests a story by ID.
/// May also include an ordinal chapter number.
#[get("/story/{id:.*}")]
async fn get_story_endpoint(
	api: ThinData<Request>, db: ThinData<Db>, path: Path<String>,
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
	id: i32, api: &Request, db: &Db, recache: bool,
) -> Result<(Story, User, Vec<Tag>)> {
	let story = db.get_story(id).await?;
	let story = check_recache!(story, recache, app);
	match story {
		Some(story) => {
			let user = request_user(story.author_id, api, db, recache).await?;
			let tag_links = db.get_tag_links(story.id).await?;
			let mut tags = Vec::with_capacity(tag_links.len());
			for link in tag_links {
				let tag = db
					.get_tag(link.tag_id)
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
			db.insert_user(&user).await?;
			let story = Story::try_from(api.data)?;
			db.insert_story(&story).await?;
			db.remove_tag_links(id).await?;
			let mut tags = Vec::with_capacity(api_tags.len());
			for tag in api_tags {
				let tag = Tag::try_from(tag.clone())?;
				db.insert_tag(&tag).await?;
				db.insert_tag_link(id, tag.id).await?;
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
