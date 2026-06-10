//! Common structs used in other modules.

use chrono::{DateTime, Utc};
use core::str;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::fmt;

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

impl From<String> for TagType {
	/// Converts a Fimfiction API response string for tag type into [TagType]
	///
	/// #### Panics
	///
	/// Panics if Fimfiction returns a value not present.
	fn from(value: String) -> Self {
		match value.as_str() {
			"character" => TagType::Character,
			"genre" => TagType::Genre,
			"rating" => TagType::Rating,
			"content" => TagType::Content,
			"series" => TagType::Series,
			"warning" => TagType::Warning,
			"universe" => TagType::Universe,
			_ => unreachable!(), // This should never happen, but still want to add something here later.
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

/// Embed parameter options
#[derive(Debug, Clone, Default)]
pub(crate) struct Parameters {
	pub(crate) cover: Option<Cover>,
	pub(crate) color: Option<Color>,
	pub(crate) refresh: bool,
	pub(crate) stats: bool,
	pub(crate) tags: bool,
}

/// Supported image options for embeds
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Cover {
	Founder,
	Story,
	User,
	None,
}

impl fmt::Display for Cover {
	/// Returns a string representation of a cover enum
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let text = match self {
			Cover::Founder => "founder",
			Cover::Story => "story",
			Cover::User => "user",
			Cover::None => "none",
		};
		write!(f, "{text}")
	}
}

/// Supported color options for embeds
#[derive(Debug, Clone)]
pub(crate) enum Color {
	Custom(String),
	Founder,
	Random,
	Modulo,
	Story,
	User,
	None,
}

impl fmt::Display for Color {
	/// Returns a string representation of a color enum
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let text = match self {
			Color::Custom(color) => color.as_str(),
			Color::Founder => "founder",
			Color::Random => "random",
			Color::Modulo => "modulo",
			Color::Story => "story",
			Color::User => "user",
			Color::None => "none",
		};
		write!(f, "{text}")
	}
}
