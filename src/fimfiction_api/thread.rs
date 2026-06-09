//! A group [thread] resource.
//!
//! [thread]: https://www.fimfiction.net/developers/api/v2/docs/resources#group_thread

use super::{ApiDebug, ApiMeta, RelationshipData};
use crate::fimfiction_api::ApiIncluded;
use serde::{Deserialize, Serialize};

/// A full thread object as returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ThreadApi<T = u32> {
	pub(crate) data: Vec<ThreadData<T>>,
	pub(crate) included: Vec<ApiIncluded<T>>,
	pub(crate) links: ThreadLinks,
	pub(crate) uri: String,
	pub(crate) method: String,
	pub(crate) debug: ApiDebug,
}

/// All properties of a thread.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ThreadData<T = u32> {
	pub(crate) id: String,
	pub(crate) r#type: String,
	pub(crate) attributes: ThreadAttributes<T>,
	pub(crate) relationships: ThreadRelationships,
	pub(crate) meta: ApiMeta,
}

/// Self-contained properties of a thread.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ThreadAttributes<T = u32> {
	pub(crate) title: String,
	pub(crate) num_posts: T,
	pub(crate) date_created: String,
	pub(crate) date_last_post: String,
	pub(crate) sticky: bool,
	pub(crate) locked: bool,
}

/// Relational properties of a thread.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ThreadRelationships {
	pub(crate) creator: RelationshipData,
	pub(crate) group: RelationshipData,
	pub(crate) last_poster: RelationshipData,
}

/// Links to the first and last posts in a thread.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ThreadLinks {
	pub(crate) first: String,
	pub(crate) last: String,
}
