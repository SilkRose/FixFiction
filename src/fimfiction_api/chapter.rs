//! A story [chapter] resource.
//!
//! [chapter]: https://www.fimfiction.net/developers/api/v2/docs/resources#chapter

use super::{ApiDebug, ApiLinks, ApiMeta, RelationshipData};
use crate::fimfiction_api::ApiIncluded;
use serde::{Deserialize, Serialize};

/// A full chapter object as returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChapterApi<T = u32> {
	pub data: ChapterData<T>,
	pub included: Vec<ApiIncluded<T>>,
	pub uri: String,
	pub method: String,
	pub debug: ApiDebug,
}

/// All properties of a chapter.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChapterData<T = u32> {
	pub id: String,
	pub r#type: String,
	pub attributes: ChapterAttributes<T>,
	pub relationships: ChapterRelationship,
	pub links: ApiLinks,
	pub meta: ApiMeta,
}

/// Relational properties of a chapter.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChapterRelationship {
	story: RelationshipData,
}

/// Self-contained properties of a chapter.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChapterAttributes<T = u32> {
	pub chapter_number: T,
	pub title: String,
	pub published: bool,
	pub num_words: T,
	pub num_views: T,
	pub date_published: String,
	pub date_modified: String,
	pub authors_note_position: String,
}
