use crate::{html_template::embed_html_template, structs::EmbedData};

pub fn error_html_template(endpoint: &str, link: String, errors: String) -> String {
	let link = format!("https://www.fimfiction.net/{endpoint}/{link}");
	let desc = format!(
		"{errors}\n\nThe link above still redirects to Fimfiction. If this error is in error, please report it to Silk Rose on Fimfction, or on the FixFiction GitHub issues page."
	);
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
