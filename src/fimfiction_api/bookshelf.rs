//! A [bookshelf] resource.
//!
//! [bookshelf]: https://www.fimfiction.net/developers/api/v2/docs/resources#bookshelf

use super::{ApiDebug, ApiLinks, ApiMeta, RelationshipData};
use crate::fimfiction_api::ApiIncluded;
use serde::{Deserialize, Serialize};

/// A full bookshelf object as returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BookshelfApi<T = u32> {
	pub(crate) data: BookshelfData<T>,
	pub(crate) included: Vec<ApiIncluded<T>>,
	pub(crate) uri: String,
	pub(crate) method: String,
	pub(crate) debug: ApiDebug,
}

/// All properties of a bookshelf.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BookshelfData<T = u32> {
	pub(crate) id: String,
	pub(crate) r#type: String,
	pub(crate) attributes: BookshelfAttributes<T>,
	pub(crate) relationships: Option<BookshelfRelationship>,
	pub(crate) links: ApiLinks,
	pub(crate) meta: ApiMeta,
}

/// Self-contained properties of a bookshelf.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BookshelfAttributes<T = u32> {
	pub(crate) name: String,
	pub(crate) privacy: String,
	pub(crate) description: String,
	pub(crate) color: String,
	pub(crate) icon: BookshelfIcon,
	pub(crate) num_stories: T,
	pub(crate) num_unread: Option<T>,
	pub(crate) track_unread: bool,
	pub(crate) quick_add: bool,
	pub(crate) email_on_update: bool,
	pub(crate) date_created: String,
	pub(crate) date_modified: String,
	pub(crate) order: T,
}

/// The icon selected to represent a bookshelf.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BookshelfIcon {
	pub(crate) name: String,
	pub(crate) r#type: String,
	pub(crate) data: String,
}

/// Relational properties of a bookshelf.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BookshelfRelationship {
	pub(crate) user: RelationshipData,
}
