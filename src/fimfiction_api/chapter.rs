//! A story [chapter] resource.
//!
//! [chapter]: https://www.fimfiction.net/developers/api/v2/docs/resources#chapter

use super::{ApiDebug, ApiLinks, ApiMeta, RelationshipData};
use crate::fimfiction_api::ApiIncluded;
use serde::{Deserialize, Serialize};

/// A full chapter object as returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ChapterApi<T = u32> {
	pub(crate) data: ChapterData<T>,
	pub(crate) included: Vec<ApiIncluded<T>>,
	pub(crate) uri: String,
	pub(crate) method: String,
	pub(crate) debug: ApiDebug,
}

/// All properties of a chapter.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ChapterData<T = u32> {
	pub(crate) id: String,
	pub(crate) r#type: String,
	pub(crate) attributes: ChapterAttributes<T>,
	pub(crate) relationships: ChapterRelationship,
	pub(crate) links: ApiLinks,
	pub(crate) meta: ApiMeta,
}

/// Relational properties of a chapter.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ChapterRelationship {
	story: RelationshipData,
}

/// Self-contained properties of a chapter.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ChapterAttributes<T = u32> {
	pub(crate) chapter_number: T,
	pub(crate) title: String,
	pub(crate) published: bool,
	pub(crate) num_words: T,
	pub(crate) num_views: T,
	pub(crate) date_published: String,
	pub(crate) date_modified: String,
	pub(crate) authors_note_position: String,
}
