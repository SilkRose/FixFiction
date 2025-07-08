use chrono::{DateTime, Utc};
use core::str;
use pony::http::Request;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "content_rating", rename_all = "lowercase")]
pub enum ContentRating {
	Everyone,
	Teen,
	Mature,
}

impl From<String> for ContentRating {
	fn from(value: String) -> Self {
		match value.as_str() {
			"everyone" => ContentRating::Everyone,
			"teen" => ContentRating::Teen,
			"mature" => ContentRating::Mature,
			_ => unreachable!(), // This should never happen, but still want to add something here later.
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "completion_status", rename_all = "lowercase")]
pub enum CompletionStatus {
	Incomplete,
	Complete,
	Hiatus,
	Cancelled,
}

impl From<String> for CompletionStatus {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "tag_type", rename_all = "lowercase")]
pub enum TagType {
	Character,
	Genre,
	Rating,
	Content,
	Series,
	Warning,
	Universe,
}

impl From<String> for TagType {
	fn from(value: String) -> Self {
		match value.as_str() {
			"character" => TagType::Character,
			"genre" => TagType::Genre,
			"rating" => TagType::Rating,
			"content" => TagType::Content,
			"series" => TagType::Series,
			"warning" => TagType::Warning,
			"universe" => TagType::Universe,
			_ => unreachable!(), // This should never happen, but still want to add something here later.
		}
	}
}

#[derive(Debug, Clone)]
pub struct Story {
	pub id: i32,
	pub title: String,
	pub short_description: String,
	pub description: String,
	pub published: bool,
	pub link: String,
	pub cover_url: Option<String>,
	pub color_hex: String,
	pub views: i32,
	pub total_views: i32,
	pub words: i32,
	pub chapters: i32,
	pub comments: i32,
	pub rating: i32,
	pub completion_status: CompletionStatus,
	pub content_rating: ContentRating,
	pub tags: String,
	pub likes: i32,
	pub dislikes: i32,
	pub author_id: i32,
	pub date_modified: DateTime<Utc>,
	pub date_updated: DateTime<Utc>,
	pub date_published: DateTime<Utc>,
	pub date_cached: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OEmbed {
	pub r#type: String,
	pub version: u32,
	pub provider_name: String,
	pub provider_url: String,
	pub title: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub author_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub author_url: Option<String>,
	pub cache_age: u32,
	pub html: String,
}

#[derive(Debug, Clone)]
pub struct User {
	pub id: i32,
	pub name: String,
	pub bio: String,
	pub link: String,
	pub followers: i32,
	pub stories: i32,
	pub blogs: i32,
	pub profile_pic_url: Option<String>,
	pub color_hex: String,
	pub date_joined: DateTime<Utc>,
	pub date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Blog {
	pub id: i32,
	pub title: String,
	pub content: String,
	pub link: String,
	pub comments: i32,
	pub views: i32,
	pub author_id: i32,
	pub tags: String,
	pub story_id: Option<i32>,
	pub date_posted: DateTime<Utc>,
	pub date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Chapter {
	pub id: i32,
	pub story_id: i32,
	pub chapter_num: i32,
	pub title: String,
	pub link: String,
	pub views: i32,
	pub words: i32,
	pub date_published: DateTime<Utc>,
	pub date_modified: DateTime<Utc>,
	pub date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Parameters {
	#[serde(alias = "image")]
	pub cover: Option<Cover>,
	pub color: Option<Color>,
	#[serde(default)]
	pub refresh: bool,
	#[serde(default)]
	pub stats: bool,
	#[serde(default)]
	pub tags: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase", try_from = "String")]
pub enum Cover {
	Story,
	User,
	None,
}

impl TryFrom<String> for Cover {
	type Error = &'static str;
	fn try_from(value: String) -> Result<Self, Self::Error> {
		match value.to_lowercase().as_str() {
			"story" => Ok(Cover::Story),
			"user" => Ok(Cover::User),
			"none" => Ok(Cover::None),
			_ => Err("invalid cover value"),
		}
	}
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase", try_from = "String")]
pub enum Color {
	Custom(String),
	Random,
	Modulo,
	Story,
	User,
	None,
}

impl From<String> for Color {
	fn from(value: String) -> Self {
		match value.to_lowercase().as_str() {
			"ran" | "random" => Color::Random,
			"mod" | "modulo" => Color::Modulo,
			"story" => Color::Story,
			"user" => Color::User,
			"none" => Color::None,
			_ => Color::Custom(value.to_lowercase()),
		}
	}
}

#[derive(Debug, Clone)]
pub struct AppState {
	pub api: Request,
	pub db: Pool<Postgres>,
	pub gc_interval: u64,
	pub cache_max_age: i64,
	pub cache_recache_age: i64,
}
