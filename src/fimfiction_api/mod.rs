//! Submodule for deserializing data from the [Fimfiction API].
//!
//! [Fimfiction API]: https://www.fimfiction.net/developers/api/v2/docs

use crate::fimfiction_api::{
	chapter::ChapterData, group::GroupData, story::StoryData, tag::TagData, user::UserData,
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::error::Error;

pub(crate) mod blog;
pub(crate) mod bookshelf;
pub(crate) mod chapter;
pub(crate) mod error;
pub(crate) mod group;
pub(crate) mod story;
pub(crate) mod tag;
pub(crate) mod thread;
pub(crate) mod user;

/// Optional resources to include in a request which the API may not return, or only return in truncated form, by default.
///
/// Types of resources include:
/// - The author of a story or blog post.
/// - The chapters of a story.
/// - The story linked to by a blog post.
/// - The tags of a story.
/// - The parent group of a group thread.
#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub(crate) enum ApiIncluded<T = u32> {
	Author(UserData<T>),
	Chapter(ChapterData<T>),
	Story(StoryData<T>),
	Tag(TagData<T>),
	Group(GroupData<T>),
}

/// A link to find the resource on the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ApiLinks {
	#[serde(rename = "self")]
	pub(crate) link: String,
}

/// A link to find the resource on the Fimfiction website.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ApiMeta {
	pub(crate) url: String,
}

/// Debug information returned by the API. Currently only contains the duration of the request.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ApiDebug {
	pub(crate) duration: String,
}

/// A color attribute as returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AttributesColor {
	pub(crate) hex: String,
	pub(crate) rgb: (u8, u8, u8),
}

/// A vector of relationship objects returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RelationshipDataVec {
	pub(crate) data: Vec<DataType>,
}

/// A relationship object returned by the API.
/// For example, the author of a story.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RelationshipData {
	pub(crate) data: DataType,
}

/// A generic "data" object returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DataType {
	pub(crate) r#type: String,
	pub(crate) id: String,
}

/// Converts a user-agent and a bearer token into a HeaderMap appropriate for use with the API.
pub(crate) fn fimfic_api_headers(
	user_agent: Option<&str>, token: &str,
) -> Result<HeaderMap, Box<dyn Error>> {
	let mut headers = HeaderMap::new();
	if let Some(user_agent) = user_agent {
		headers.insert(USER_AGENT, HeaderValue::from_str(user_agent)?);
	}
	headers.insert(
		AUTHORIZATION,
		HeaderValue::from_str(&format!("Bearer {token}"))?,
	);
	headers.insert(
		CONTENT_TYPE,
		HeaderValue::from_static("application/vnd.api+json"),
	);
	Ok(headers)
}
