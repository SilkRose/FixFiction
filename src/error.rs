//! Format errors in HTML.

use crate::html_template::{EmbedData, embed_html_template};
use crate::utility::LOG;
use actix_web::error::InternalError;
use actix_web::http::StatusCode;
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, ResponseError};
use std::error::Error as StdError;
use std::fmt::Display;
use std::result::Result as StdResult;

pub(crate) type Error = Box<dyn StdError>;
pub(crate) type Result<T, E = Error> = StdResult<T, E>;
pub(crate) type EmbedResult<T> = Result<T, actix_web::Error>;

pub(crate) trait EmbedError<T> {
	fn map_embed_err(self, endpoint: &str, link: &str) -> EmbedResult<T>;
}

impl<T> EmbedError<T> for Result<T> {
	fn map_embed_err(self, endpoint: &str, link: &str) -> EmbedResult<T> {
		match self {
			Ok(value) => Ok(value),
			Err(error) => {
				let body = error_html_template(endpoint, link, error.to_string());
				let res = EmbedErrorType::new(body).error_response();
				let err = InternalError::from_response(error, res);
				Err(err.into())
			}
		}
	}
}

#[derive(Debug)]
pub(crate) struct EmbedErrorType {
	inner: String,
}

impl EmbedErrorType {
	pub fn new(body: String) -> Self {
		EmbedErrorType { inner: body }
	}
}

impl std::error::Error for EmbedErrorType {}

impl Display for EmbedErrorType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.inner)
	}
}

impl ResponseError for EmbedErrorType {
	fn status_code(&self) -> StatusCode {
		StatusCode::OK
	}

	fn error_response(&self) -> HttpResponse {
		HttpResponse::build(self.status_code())
			.content_type(ContentType::html())
			.body(self.to_string())
	}
}

/// Formats errors to an HTML string for embedding.
pub(crate) fn error_html_template(endpoint: &str, link: &str, errors: String) -> String {
	let link = format!("https://www.fimfiction.net/{endpoint}/{link}");
	let desc = format!(
		"{errors}\n\nThe link above still redirects to Fimfiction. If this error is in error, please report it to Silk Rose on Fimfiction, or on the FixFiction GitHub issues page."
	);
	let msg = format!("{errors} -- Link: {link}");
	if let Err(e) = LOG.error(&msg) {
		eprintln!("Failed to log error: {e}")
	}
	let data = EmbedData {
		title: String::from("Redirect to Fimfiction"),
		description: desc,
		link,
		color: Some(String::from("f5b7d0")),
		cover: Some(String::from(
			"https://derpicdn.net/img/view/2012/6/18/6782.jpg",
		)),
		site_name: String::from("FixFiction Issues Page"),
		site_url: String::from("https://github.com/SilkRose/FixFiction/issues"),
		errors: Vec::default(),
		user_name: Some(String::from("Silk Rose's Fimfiction Profile")),
		user_link: Some(String::from("https://www.fimfiction.net/user/237915/")),
		html_comment: Some(String::from(
			"Error embed image by MegaSweet: https://derpibooru.org/images/6782",
		)),
		open_graph_type: String::from("book"),
		open_graph_property: Some(String::from("book:author")),
	};
	embed_html_template(data)
}
