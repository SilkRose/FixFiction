use crate::cookie::{CloudFlareData, get_cookie};
use chrono::{DateTime, Utc};
use core::str;
use pony::http::Request;
use reqwest::header::{COOKIE, HeaderValue};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Type};
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::error::Error;
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

impl PartialOrd for TagType {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(Self::cmp(self, other))
	}
}

impl Ord for TagType {
	fn cmp(&self, other: &Self) -> Ordering {
		macro_rules! to_int {
			($tag:ident) => {
				match *$tag {
					TagType::Rating => 1,
					TagType::Series => 2,
					TagType::Universe => 3,
					TagType::Warning => 4,
					TagType::Genre => 5,
					TagType::Content => 6,
					TagType::Character => 7,
				}
			};
		}

		let this = to_int!(self);
		let other = to_int!(other);
		Ord::cmp(&this, &other)
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
	pub likes: i32,
	pub dislikes: i32,
	pub author_id: i32,
	pub date_modified: DateTime<Utc>,
	pub date_updated: DateTime<Utc>,
	pub date_published: DateTime<Utc>,
	pub date_cached: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Tag {
	pub id: i32,
	pub name: String,
	pub tag_type: TagType,
	pub old_id: Option<String>,
	pub link: String,
	pub date_cached: DateTime<Utc>,
}

impl PartialEq for Tag {
	fn eq(&self, other: &Self) -> bool {
		matches!(Ord::cmp(self, other), Ordering::Equal)
	}
}

impl Eq for Tag {}

impl PartialOrd for Tag {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(Self::cmp(self, other))
	}
}

impl Ord for Tag {
	fn cmp(&self, other: &Self) -> Ordering {
		let cmp = Ord::cmp(&self.tag_type, &other.tag_type);
		if cmp != Ordering::Equal {
			return cmp;
		}

		Ord::cmp(&self.id, &other.id)
	}
}

#[derive(Debug, Clone)]
pub struct TagLink {
	pub story_id: i32,
	pub tag_id: i32,
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

#[derive(Debug, Clone)]
pub struct ThreadReturn {
	pub thread: Thread,
	pub creator: User,
	pub last_poster: User,
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
	pub cf_data: CloudFlareData,
	pub gc_interval: u64,
	pub cache_max_age: i64,
	pub cache_recache_age: i64,
}

impl AppState {
	pub async fn refresh_cookie(&mut self) -> Result<(), Box<dyn Error>> {
		self.cf_data = get_cookie(&self.api.client).await?;
		self.insert_cookie()?;
		Ok(())
	}

	pub fn insert_cookie(&mut self) -> Result<(), Box<dyn Error>> {
		for cookie in &self.cf_data.cookies {
			self.api
				.headers
				.insert(COOKIE, HeaderValue::from_str(cookie)?);
		}
		Ok(())
	}
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
