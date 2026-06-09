//! A story [tag] resource.
//!
//! [tag]: https://www.fimfiction.net/developers/api/v2/docs/resources#story_tag

use super::ApiDebug;
use serde::{Deserialize, Serialize};

/// A full tag object as returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TagApi<T = u32> {
	pub(crate) data: Vec<TagData<T>>,
	pub(crate) included: Vec<()>,
	pub(crate) uri: String,
	pub(crate) method: String,
	pub(crate) debug: ApiDebug,
}

/// All properties of a tag.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TagData<T = u32> {
	pub(crate) id: String,
	pub(crate) r#type: String,
	pub(crate) attributes: TagAttributes<T>,
	pub(crate) meta: TagMeta,
}

/// Self-contained properties of a tag.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TagAttributes<T = u32> {
	pub(crate) name: String,
	pub(crate) r#type: String,
	pub(crate) num_stories: T,
}

/// Self-referential properties of a tag.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TagMeta {
	pub(crate) old_id: String,
	pub(crate) url: String,
}
