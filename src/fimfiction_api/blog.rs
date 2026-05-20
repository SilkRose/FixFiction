//! A [blog] post resource.
//! 
//! [blog]: https://www.fimfiction.net/developers/api/v2/docs/resources#blog_post

use super::{ApiDebug, ApiMeta, RelationshipData};
use crate::fimfiction_api::ApiIncluded;
use serde::{Deserialize, Serialize};

/// A full blog object as returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlogApi<T = u32> {
	pub data: BlogData<T>,
	pub included: Vec<ApiIncluded<T>>,
	pub uri: String,
	pub method: String,
	pub debug: ApiDebug,
}

/// All properties of a blog.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlogData<T = u32> {
	pub id: String,
	pub r#type: String,
	pub attributes: BlogAttributes<T>,
	pub relationships: BlogRelationships,
	pub meta: ApiMeta,
}

/// Self-contained properties of a blog.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlogAttributes<T = u32> {
	pub title: String,
	pub date_posted: String,
	pub content: String,
	pub num_views: T,
	pub num_comments: T,
	pub site_post: bool,
	pub tags: Vec<String>,
}

/// Relational properties of a blog
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlogRelationships {
	pub tagged_story: RelationshipData,
}
