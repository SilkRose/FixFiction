use crate::structs::EmbedData;
use url::form_urlencoded;

pub fn embed_html_template(embed: EmbedData) -> String {
	let mut text = String::new();
	text.push_str(r#"<!DOCTYPE html><html lang="en"><head>"#);
	text.push_str("<!-- FixFiction: https://github.com/SilkRose/FixFiction -->");
	text.push_str("<!-- Pinkie Pie is best pony! -->");
	if let Some(comment) = embed.html_comment {
		text.push_str(&format!("<!-- {comment} -->"));
	}
	if let Some(color) = embed.color {
		text.push_str(&format!(
			r##"<meta name="theme-color" content="#{color}" />"##
		));
	}
	text.push_str(&format!(
		r#"<link rel="canonical" href="{}" />"#,
		embed.link
	));
	text.push_str(&format!(
		r#"<meta http-equiv="refresh" content="0;url={}" />"#,
		embed.link
	));
	text.push_str(&format!(
		r#"<meta property="og:title" content="{}" />"#,
		embed.title
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{}" />"#,
		embed.description
	));
	if let Some(cover) = embed.cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
	text.push_str(&format!(
		r#"<meta property="og:url" content="{}" />"#,
		embed.link
	));
	text.push_str(&format!(
		r#"<meta property="og:type" content="{}" />"#,
		embed.open_graph_type
	));
	if let Some(property) = embed.open_graph_property {
		if let Some(ref username) = embed.user_name {
			text.push_str(&format!(
				r#"<meta property="{property}" content="{username}" />"#
			))
		}
	};
	let site_name = if !embed.errors.is_empty() {
		format!("{}\n{}", embed.site_name, embed.errors)
	} else {
		embed.site_name
	};
	text.push_str(&format!(
		r#"<meta property="og:site_name" content="{site_name}" />"#
	));
	text.push_str(r#"<meta property="twitter:site" content="fimfiction" />"#);
	text.push_str(r#"<meta property="twitter:card" content="summary" />"#);
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair("type", "rich");
	encode.append_pair("version", "1");
	encode.append_pair("provider_name", &site_name);
	encode.append_pair("provider_url", &embed.site_url);
	encode.append_pair("title", &embed.title);
	if let Some(ref username) = embed.user_name {
		encode.append_pair("author_name", username);
	}
	if let Some(ref user_link) = embed.user_link {
		encode.append_pair("author_url", user_link);
	}
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	let encode = encode.finish();
	text.push_str(&format!(
		r#"<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed?{encode}" title="{}" />"#, embed.title));
	text.push_str(r#"</head><body></body></html>"#);
	text
}
