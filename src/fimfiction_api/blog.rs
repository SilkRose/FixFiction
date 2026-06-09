//! A [blog] post resource.
//!
//! [blog]: https://www.fimfiction.net/developers/api/v2/docs/resources#blog_post

use super::{ApiDebug, ApiMeta, RelationshipData};
use crate::fimfiction_api::ApiIncluded;
use serde::{Deserialize, Serialize};

/// A full blog object as returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BlogApi<T = u32> {
	pub(crate) data: BlogData<T>,
	pub(crate) included: Vec<ApiIncluded<T>>,
	pub(crate) uri: String,
	pub(crate) method: String,
	pub(crate) debug: ApiDebug,
}

/// All properties of a blog.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BlogData<T = u32> {
	pub(crate) id: String,
	pub(crate) r#type: String,
	pub(crate) attributes: BlogAttributes<T>,
	pub(crate) relationships: BlogRelationships,
	pub(crate) meta: ApiMeta,
}

/// Self-contained properties of a blog.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BlogAttributes<T = u32> {
	pub(crate) title: String,
	pub(crate) date_posted: String,
	pub(crate) content: String,
	pub(crate) num_views: T,
	pub(crate) num_comments: T,
	pub(crate) site_post: bool,
	pub(crate) tags: Vec<String>,
}

/// Relational properties of a blog
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BlogRelationships {
	pub(crate) tagged_story: RelationshipData,
}
