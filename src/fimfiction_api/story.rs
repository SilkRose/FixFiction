//! A [story] resource.
//!
//! [story]: https://www.fimfiction.net/developers/api/v2/docs/resources#story

use super::{ApiDebug, ApiLinks, ApiMeta, AttributesColor, RelationshipData, RelationshipDataVec};
use crate::fimfiction_api::ApiIncluded;
use serde::{Deserialize, Serialize};

/// A full story object as returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StoryApi<T = u32> {
	pub(crate) data: StoryData<T>,
	pub(crate) included: Vec<ApiIncluded<T>>,
	pub(crate) uri: String,
	pub(crate) method: String,
	pub(crate) debug: ApiDebug,
}

/// All properties of a story.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StoryData<T = u32> {
	pub(crate) id: String,
	pub(crate) r#type: String,
	pub(crate) attributes: StoryAttributes<T>,
	pub(crate) relationships: StoryRelationships,
	pub(crate) links: ApiLinks,
	pub(crate) meta: ApiMeta,
}

/// Self-contained properties of a story.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StoryAttributes<T = u32> {
	pub(crate) title: String,
	pub(crate) short_description: String,
	pub(crate) description: String,
	pub(crate) description_html: String,
	pub(crate) date_modified: String,
	pub(crate) date_updated: Option<String>,
	pub(crate) date_published: Option<String>,
	pub(crate) published: bool,
	pub(crate) cover_image: Option<AttributesCoverImage>,
	pub(crate) color: AttributesColor,
	pub(crate) num_views: T,
	pub(crate) total_num_views: T,
	pub(crate) num_words: T,
	pub(crate) num_chapters: T,
	pub(crate) num_comments: T,
	pub(crate) rating: T,
	pub(crate) status: String,
	pub(crate) submitted: bool,
	pub(crate) completion_status: String,
	pub(crate) content_rating: String,
	pub(crate) num_likes: i32,
	pub(crate) num_dislikes: i32,
}

/// The image selected as the cover for a story, in various sizes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AttributesCoverImage {
	pub(crate) thumbnail: String,
	pub(crate) medium: String,
	pub(crate) large: String,
	pub(crate) full: String,
}

/// Relational properties of a story.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StoryRelationships {
	pub(crate) author: RelationshipData,
	pub(crate) tags: RelationshipDataVec,
	pub(crate) prequel: Option<RelationshipData>,
}
