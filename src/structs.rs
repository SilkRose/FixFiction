use chrono::{DateTime, Utc};
use core::str;
use pony::http::Request;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Type};
use std::fmt;

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

#[derive(Debug, Clone)]
pub struct Group {
	pub id: i32,
	pub name: String,
	pub description: String,
	pub link: String,
	pub members: i32,
	pub stories: i32,
	pub founder_id: i32,
	pub nsfw: bool,
	pub open: bool,
	pub hidden: bool,
	pub icon_url: Option<String>,
	pub date_created: DateTime<Utc>,
	pub date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Bookshelf {
	pub id: i32,
	pub name: String,
	pub description: String,
	pub link: String,
	pub color: String,
	pub icon_url: String,
	pub stories: i32,
	pub num_unread: Option<i32>,
	pub track_unread: bool,
	pub quick_add: bool,
	pub email_update: bool,
	pub user_id: Option<i32>,
	pub order_pos: i32,
	pub date_created: DateTime<Utc>,
	pub date_modified: DateTime<Utc>,
	pub date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Thread {
	pub id: i32,
	pub group_id: i32,
	pub creator_id: i32,
	pub last_poster_id: i32,
	pub title: String,
	pub link: String,
	pub posts: i32,
	pub sticky: bool,
	pub locked: bool,
	pub date_created: DateTime<Utc>,
	pub date_last_post: DateTime<Utc>,
	pub date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct Parameters {
	pub cover: Option<Cover>,
	pub color: Option<Color>,
	pub refresh: bool,
	pub stats: bool,
	pub tags: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cover {
	Founder,
	Story,
	User,
	None,
}

impl fmt::Display for Cover {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let text = match self {
			Cover::Founder => "founder",
			Cover::Story => "story",
			Cover::User => "user",
			Cover::None => "none",
		};
		write!(f, "{text}")
	}
}

#[derive(Debug, Clone)]
pub enum Color {
	Custom(String),
	Founder,
	Random,
	Modulo,
	Story,
	User,
	None,
}

impl fmt::Display for Color {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let text = match self {
			Color::Custom(color) => color.as_str(),
			Color::Founder => "founder",
			Color::Random => "random",
			Color::Modulo => "modulo",
			Color::Story => "story",
			Color::User => "user",
			Color::None => "none",
		};
		write!(f, "{text}")
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

#[derive(Debug, Clone)]
pub struct EmbedData {
	pub title: String,
	pub description: String,
	pub link: String,
	pub color: Option<String>,
	pub cover: Option<String>,
	pub site_name: String,
	pub site_url: String,
	pub errors: Vec<String>,
	pub user_name: Option<String>,
	pub user_link: Option<String>,
	pub html_comment: Option<String>,
	pub open_graph_type: String,
	pub open_graph_property: Option<String>,
}
