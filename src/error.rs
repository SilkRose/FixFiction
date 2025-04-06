use url::form_urlencoded;

pub fn error_html_template(endpoint: &str, link: String, errors: String) -> String {
	let title = "Redirect to Fimfiction";
	let site_name = "FixFiction Issues Page";
	let author_name = "Silk Rose's Fimfiction Profile";
	let link = format!("https://www.fimfiction.net/{endpoint}/{link}");
	let desc = "\n\nThe link above still redirects to Fimfiction. If this error is in error, report it to Silk Rose on Fimfction, or on the FixFiction GitHub issues page.";
	let author_url = "https://www.fimfiction.net/user/237915/";
	let site_url = "https://github.com/SilkRose/FixFiction/issues";
	let image = "https://derpicdn.net/img/view/2012/6/18/6782.jpg";
	let mut text = String::new();
	text.push_str(r#"<!DOCTYPE html><html lang="en"><head>"#);
	text.push_str("<!-- FixFiction: https://github.com/SilkRose/FixFiction -->");
	text.push_str("<!-- Pinkie Pie is best pony! -->");
	text.push_str("<!-- Error embed image by MegaSweet: https://derpibooru.org/images/6782 -->");
	text.push_str(r##"<meta name="theme-color" content="#F5B7D0" />"##);
	text.push_str(&format!(r#"<link rel="canonical" href="{link}" />"#));
	text.push_str(&format!(
		r#"<meta http-equiv="refresh" content="0;url={link}" />"#
	));
	text.push_str(&format!(
		r#"<meta property="og:title" content="{title}" />"#,
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{errors}{desc}" />"#
	));
	text.push_str(&format!(
		r#"<meta property="og:image" content="{image}" />"#
	));
	text.push_str(&format!(r#"<meta property="og:url" content="{link}" />"#));
	text.push_str(r#"<meta property="og:type" content="book" />"#);
	text.push_str(&format!(
		r#"<meta property="book:author" content="{author_url}" />"#,
	));
	text.push_str(&format!(
		r#"<meta property="og:site_name" content="{site_name}" />"#
	));
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair("type", "rich");
	encode.append_pair("version", "1");
	encode.append_pair("provider_name", site_name);
	encode.append_pair("provider_url", site_url);
	encode.append_pair("title", title);
	encode.append_pair("author_name", author_name);
	encode.append_pair("author_url", author_url);
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	let encode = encode.finish();
	text.push_str(&format!(
		r#"<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed?{encode}" title="{author_name}" />"#));
	text.push_str(r#"</head><body></body></html>"#);
	text
}
