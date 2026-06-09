//! Building HTML and oEmbed data for embedding.

use crate::{structs::EmbedData, utility::LOG};
use maud::{DOCTYPE, PreEscaped, html};
use url::form_urlencoded;

/// Builds an HTML string to present as the embedded page.
pub(crate) fn embed_html_template(embed: EmbedData) -> String {
	for warning in &embed.errors {
		let msg = format!("{warning} -- Link: {}", embed.link);
		if let Err(e) = LOG.warn(&msg) {
			eprintln!("{e}")
		}
	}
	html! {
		(DOCTYPE) html lang = "en" {
			head {
				(PreEscaped ("<!-- FixFiction: https://github.com/SilkRose/FixFiction -->"));
				(PreEscaped ("<!-- Pinkie Pie is best pony! -->"));
				@if let Some(comment) = embed.html_comment {
					(PreEscaped (format!("<!-- {comment} -->")));
				}
				@if let Some(color) = embed.color {
					meta name = "theme-color" content = { "#" (color) } "/";
				}
				link rel = "canonical" href = (embed.link) "/";
				script {
					(PreEscaped (format!(r#"window.location.href = "{}" + window.location.hash;"#, embed.link)))
				};
				meta http-equiv = "refresh" content={ "0;url=" (embed.link) } "/";
				meta property = "og:title" content = (embed.title) "/";
				meta property = "og:description" content = (embed.description) "/";
				@if let Some(cover) = embed.cover {
					meta property = "og:image" content = (cover) "/";
				}
				meta property = "og:url" content = (embed.link) "/";
				meta property = "og:type" content = (embed.open_graph_type) "/";
				@if let Some(property) = embed.open_graph_property {
					@if let Some(ref username) = embed.user_name {
						meta property = (property) content = (username) "/";
					}
				};
				@let site_name = if !embed.errors.is_empty() {
					format!("{}\n{}", embed.site_name, embed.errors.join(", "))
				} else {
					embed.site_name
				};
				meta property = "og:site_name" content = (site_name) "/";
				meta property = "twitter:site" content = "fimfiction" "/";
				meta property = "twitter:card" content = "summary" "/";
				@let encode = encode_url(
					&site_name,
					&embed.site_url,
					&embed.title,
					embed.user_name,
					embed.user_link);
				link
					rel = "alternate"
					type = "application/json+oembed"
					href = { "https://www.fixfiction.net/oembed?" (encode) }
					title = (embed.title) "/";
				};
			body {};
		};
	}
	.into()
}

/// Builds [oEmbed](https://oembed.com/) data for embedding.
fn encode_url(
	site_name: &str, site_url: &str, title: &str, username: Option<String>,
	user_link: Option<String>,
) -> String {
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair("type", "rich");
	encode.append_pair("version", "1");
	encode.append_pair("provider_name", site_name);
	encode.append_pair("provider_url", site_url);
	encode.append_pair("title", title);
	if let Some(ref username) = username {
		encode.append_pair("author_name", username);
	}
	if let Some(ref user_link) = user_link {
		encode.append_pair("author_url", user_link);
	}
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	encode.finish()
}
