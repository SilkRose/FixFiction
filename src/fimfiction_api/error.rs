//!	An [error] returned by the API instead of the requested resource.
//!
//! [error]: https://www.fimfiction.net/developers/api/v2/docs/error-codes

use serde::{Deserialize, Serialize};

/// A list of errors returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FimficError<T = u32> {
	pub errors: Vec<FimficErrorInner<T>>,
}

/// A full individual error object.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FimficErrorInner<T = u32> {
	pub status: String,
	pub title: String,
	pub detail: Option<String>,
	pub code: T,
	pub meta: Option<ErrorMeta<T>>,
	pub links: ErrorLinks,
}

/// Properties of a potential resource related to an error.
///
/// The resource may not exist if, for example, the error was "Resource not found".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ErrorMeta<T = u32> {
	pub resource: String,
	pub id: OptionsID<T>,
}

/// The ID field of a potential resource. The API may return it as a number or a string.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum OptionsID<T> {
	Number(T),
	String(String),
}

/// Links provided by the API to add context to the error.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ErrorLinks {
	pub about: String,
}
