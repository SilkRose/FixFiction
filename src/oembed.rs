use crate::error::Result;
use actix_web::{HttpResponse, Responder, get, web::Query};
use serde::{Deserialize, Serialize};

/// OEmbed data structure for OEmbed support
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct OEmbed {
	pub(crate) r#type: String,
	pub(crate) version: u32,
	pub(crate) provider_name: String,
	pub(crate) provider_url: String,
	pub(crate) title: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) author_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) author_url: Option<String>,
	pub(crate) cache_age: u32,
	pub(crate) html: String,
}

/// oEmbed endpoint for enhanced embeds
#[get("/oembed")]
pub(crate) async fn get_oembed(query: Query<OEmbed>) -> Result<impl Responder> {
	let embed = query.into_inner();
	Ok(HttpResponse::Ok()
		.content_type("application/json+oembed")
		.json(embed))
}
