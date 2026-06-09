//! A [group] resource.
//!
//! [group]: https://www.fimfiction.net/developers/api/v2/docs/resources#group

use super::{ApiDebug, ApiLinks, ApiMeta, RelationshipData};
use crate::fimfiction_api::ApiIncluded;
use serde::{Deserialize, Serialize};

/// A full group as returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GroupApi<T = u32> {
	pub(crate) data: GroupData<T>,
	pub(crate) included: Vec<ApiIncluded<T>>,
	pub(crate) uri: String,
	pub(crate) method: String,
	pub(crate) debug: ApiDebug,
}

/// All properties of a group.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GroupData<T = u32> {
	pub(crate) id: String,
	pub(crate) r#type: String,
	pub(crate) attributes: GroupAttributes<T>,
	pub(crate) relationships: GroupRelationship,
	pub(crate) links: ApiLinks,
	pub(crate) meta: ApiMeta,
}

/// Relational properties of a group.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GroupRelationship {
	pub(crate) founder: RelationshipData,
}

/// Self-contained properties of a group.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GroupAttributes<T = u32> {
	pub(crate) name: String,
	pub(crate) num_members: T,
	pub(crate) num_stories: T,
	pub(crate) icon: GroupIcon,
	pub(crate) nsfw: bool,
	pub(crate) open: bool,
	pub(crate) hidden: bool,
	pub(crate) date_created: String,
	pub(crate) description: String,
	pub(crate) description_html: String,
}

/// The image selected as an icon for a group, optionally in various sizes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GroupIcon {
	#[serde(rename = "32")]
	pub(crate) r32: Option<String>,
	#[serde(rename = "48")]
	pub(crate) r48: Option<String>,
	#[serde(rename = "64")]
	pub(crate) r64: Option<String>,
	#[serde(rename = "96")]
	pub(crate) r96: Option<String>,
	#[serde(rename = "128")]
	pub(crate) r128: Option<String>,
	#[serde(rename = "160")]
	pub(crate) r160: Option<String>,
	#[serde(rename = "192")]
	pub(crate) r192: Option<String>,
	#[serde(rename = "256")]
	pub(crate) r256: Option<String>,
	#[serde(rename = "320")]
	pub(crate) r320: Option<String>,
	#[serde(rename = "384")]
	pub(crate) r384: Option<String>,
	#[serde(rename = "512")]
	pub(crate) r512: Option<String>,
}
