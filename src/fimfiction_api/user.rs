//! A [user] resource.
//!
//! [user]: https://www.fimfiction.net/developers/api/v2/docs/resources#user

use super::{ApiDebug, ApiLinks, ApiMeta, AttributesColor};
use serde::{Deserialize, Serialize};

/// A full user object as returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UserApi<T = u32> {
	pub(crate) data: UserData<T>,
	pub(crate) included: Vec<()>,
	pub(crate) uri: String,
	pub(crate) method: String,
	pub(crate) debug: ApiDebug,
}

/// All properties of a user.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UserData<T = u32> {
	pub(crate) id: String,
	pub(crate) r#type: String,
	pub(crate) attributes: UserAttributes<T>,
	pub(crate) links: ApiLinks,
	pub(crate) meta: ApiMeta,
}

/// Self-contained properties of a user.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UserAttributes<T = u32> {
	pub(crate) name: String,
	pub(crate) bio: String,
	pub(crate) bio_html: String,
	pub(crate) num_followers: T,
	pub(crate) num_stories: T,
	pub(crate) num_blog_posts: T,
	pub(crate) avatar: AttributesAvatar,
	pub(crate) date_last_online: Option<String>,
	pub(crate) color: AttributesColor,
	pub(crate) date_joined: String,
}

/// The image selected as an avatar for a user, optionally in various sizes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AttributesAvatar {
	#[serde(rename = "32")]
	pub(crate) r32: String,
	#[serde(rename = "48")]
	pub(crate) r48: String,
	#[serde(rename = "64")]
	pub(crate) r64: String,
	#[serde(rename = "96")]
	pub(crate) r96: String,
	#[serde(rename = "128")]
	pub(crate) r128: String,
	#[serde(rename = "160")]
	pub(crate) r160: String,
	#[serde(rename = "192")]
	pub(crate) r192: String,
	#[serde(rename = "256")]
	pub(crate) r256: String,
	#[serde(rename = "320")]
	pub(crate) r320: String,
	#[serde(rename = "384")]
	pub(crate) r384: String,
	#[serde(rename = "512")]
	pub(crate) r512: String,
}
