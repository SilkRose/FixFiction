//! [Tag] and [TagLink] data structure and related code

use crate::error::{Error, Result};
use crate::fimfiction_api::tag::TagData;
use chrono::{DateTime, Utc};
use core::str;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};

/// Fimfiction tag type data converted into a more usable structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "tag_type", rename_all = "lowercase")]
pub(crate) enum TagType {
	Character,
	Genre,
	Rating,
	Content,
	Series,
	Warning,
	Universe,
}

impl TryFrom<String> for TagType {
	type Error = Error;
	/// Converts a Fimfiction API response string for tag type into [TagType]
	fn try_from(value: String) -> Result<Self> {
		match value.as_str() {
			"character" => Ok(TagType::Character),
			"genre" => Ok(TagType::Genre),
			"rating" => Ok(TagType::Rating),
			"content" => Ok(TagType::Content),
			"series" => Ok(TagType::Series),
			"warning" => Ok(TagType::Warning),
			"universe" => Ok(TagType::Universe),
			_ => Err(format!(
				"FixFiction error: failed to parse string into completion status: {value}"
			)
			.into()),
		}
	}
}

impl PartialOrd for TagType {
	/// Sorting tags by their type
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(Self::cmp(self, other))
	}
}

impl Ord for TagType {
	/// Sorting tags by their type
	fn cmp(&self, other: &Self) -> Ordering {
		macro_rules! to_int {
			($tag:ident) => {
				match *$tag {
					TagType::Rating => 1,
					TagType::Series => 2,
					TagType::Universe => 3,
					TagType::Warning => 4,
					TagType::Genre => 5,
					TagType::Content => 6,
					TagType::Character => 7,
				}
			};
		}

		let this = to_int!(self);
		let other = to_int!(other);
		Ord::cmp(&this, &other)
	}
}

/// Fimfiction tag data converted into a more usable structure
#[derive(Debug, Clone)]
pub(crate) struct Tag {
	pub(crate) id: i32,
	pub(crate) name: String,
	pub(crate) tag_type: TagType,
	pub(crate) old_id: Option<String>,
	pub(crate) link: String,
	pub(crate) date_cached: DateTime<Utc>,
}

impl TryFrom<TagData<i32>> for Tag {
	type Error = Error;
	/// Converts Fimfiction's API response [TagData] into a [Tag]
	fn try_from(value: TagData<i32>) -> Result<Self> {
		let old_id = match value.meta.old_id.is_empty() {
			true => None,
			false => Some(value.meta.old_id),
		};
		let tag = Tag {
			id: value.id.parse()?,
			name: value.attributes.name,
			tag_type: TagType::try_from(value.attributes.r#type)?,
			old_id,
			link: value.meta.url,
			date_cached: Utc::now(),
		};
		Ok(tag)
	}
}

impl PartialEq for Tag {
	/// Checking if two tags are the same
	fn eq(&self, other: &Self) -> bool {
		matches!(Ord::cmp(self, other), Ordering::Equal)
	}
}

impl Eq for Tag {}

impl PartialOrd for Tag {
	/// Sorting tags by their type then id
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(Self::cmp(self, other))
	}
}

impl Ord for Tag {
	/// Sorting tags by their type then id
	fn cmp(&self, other: &Self) -> Ordering {
		let cmp = Ord::cmp(&self.tag_type, &other.tag_type);
		if cmp != Ordering::Equal {
			return cmp;
		}

		Ord::cmp(&self.id, &other.id)
	}
}

/// Fimfiction tag link data converted into a more usable structure
#[derive(Debug, Clone)]
pub(crate) struct TagLink {
	pub(crate) story_id: i32,
	pub(crate) tag_id: i32,
	pub(crate) date_cached: DateTime<Utc>,
}
