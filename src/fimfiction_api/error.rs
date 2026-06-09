//! An [error] returned by the API instead of the requested resource.
//!
//! [error]: https://www.fimfiction.net/developers/api/v2/docs/error-codes

use serde::{Deserialize, Serialize};

/// A list of errors returned by the API.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FimficError<T = u32> {
	pub(crate) errors: Vec<FimficErrorInner<T>>,
}

/// A full individual error object.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FimficErrorInner<T = u32> {
	pub(crate) status: String,
	pub(crate) title: String,
	pub(crate) detail: Option<String>,
	pub(crate) code: T,
	pub(crate) meta: Option<ErrorMeta<T>>,
	pub(crate) links: ErrorLinks,
}

/// Properties of a potential resource related to an error.
///
/// The resource may not exist if, for example, the error was "Resource not found".
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ErrorMeta<T = u32> {
	pub(crate) resource: String,
	pub(crate) id: OptionsID<T>,
}

/// The ID field of a potential resource. The API may return it as a number or a string.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub(crate) enum OptionsID<T> {
	Number(T),
	String(String),
}

/// Links provided by the API to add context to the error.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ErrorLinks {
	pub(crate) about: String,
}
