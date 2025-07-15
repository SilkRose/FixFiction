use crate::database::{get_group, insert_group, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::group::GroupApi;
use crate::structs::{AppState, Color, Cover, Group, Parameters, User};
use crate::user::request_user;
use crate::utility::{get_color, map_picture, parse_fimfic_response};
use crate::{check_recache, get_variant};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};
use std::error::Error;
use url::form_urlencoded;

pub async fn request_group(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Group, User), Box<dyn Error>> {
	let group = get_group(id, &app.db).await?;
	let group = check_recache!(group, recache, app);
	match group {
		Some(group) => {
			let founder = request_user(group.founder_id, app, recache).await?;
			Ok((group, founder))
		}
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/groups/{id}?include=founder");
			let api = parse_fimfic_response::<GroupApi<i32>>(&app.api, &fimfic).await?;
			let founder = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no founder included")?;
			let founder = insert_user(None, founder, &app.db).await?;
			let group = insert_group(Some(id), &api.data, &app.db).await?;
			Ok((group, founder))
		}
	}
}

pub fn group_html_template(
	group: Group, founder: User, parameters: Parameters, link: String, errors: String,
) -> String {
	let mut text = String::new();
	let founder_link = match parameters.stats {
		true => Some(format!("Founder: {}", founder.name)),
		false => None,
	};
	text.push_str(r#"<!DOCTYPE html><html lang="en"><head>"#);
	text.push_str("<!-- FixFiction: https://github.com/SilkRose/FixFiction -->");
	text.push_str("<!-- Pinkie Pie is best pony! -->");
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::User => Some(founder.color_hex),
			Color::Random => Some(get_color(None)),
			Color::Modulo | Color::Story => Some(get_color(Some(group.id))),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => Some(get_color(Some(group.id))),
				Cover::User => Some(founder.color_hex),
				Cover::None => None,
			},
			None => Some(get_color(Some(group.id))),
		},
	};
	if let Some(color) = color {
		text.push_str(&format!(
			r##"<meta name="theme-color" content="#{color}" />"##
		));
	}
	text.push_str(&format!(r#"<link rel="canonical" href="{link}" />"#));
	text.push_str(&format!(
		r#"<meta http-equiv="refresh" content="0;url={link}" />"#
	));
	text.push_str(&format!(
		r#"<meta property="og:title" content="{}" />"#,
		group.name
	));
	text.push_str(&format!(
		r#"<meta property="og:description" content="{}" />"#,
		group.description
	));
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::User | Cover::Story => map_picture(founder.profile_pic_url),
			Cover::None => None,
		},
		None => map_picture(group.icon_url).or(map_picture(founder.profile_pic_url)),
	};
	if let Some(cover) = cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
	text.push_str(&format!(r#"<meta property="og:url" content="{link}" />"#));
	text.push_str(r#"<meta property="og:type" content="profile" />"#);
	if let Some(ref founder_link) = founder_link {
		text.push_str(&format!(
			r#"<meta property="profile:username" content="{founder_link}" />"#,
		));
	}
	let mut site_name = if parameters.stats {
		let time = group.date_created.format("%a %b %e %Y").to_string();
		let mature = match group.nsfw {
			true => "Not safe for work: 🔞",
			false => "Safe for work: ✅",
		};
		let open = match group.open {
			true => "Open submissions: ✅",
			false => "Closed submissions: 🚫",
		};
		format!(
			"Fimfiction - Created: {time} 📅\nMembers: {} 👥 Stories: {} 📚\n{open} {mature}",
			format_number_unit_metric(group.members as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(group.stories as f64, FormatType::MetricPrefix, 1).unwrap(),
		)
	} else {
		"Fimfiction".to_string()
	};
	if !errors.is_empty() {
		site_name = format!("{site_name}\n{errors}");
	}
	text.push_str(&format!(
		r#"<meta property="og:site_name" content="{site_name}" />"#
	));
	text.push_str(r#"<meta property="twitter:site" content="fimfiction" />"#);
	text.push_str(r#"<meta property="twitter:card" content="summary" />"#);
	let mut encode = form_urlencoded::Serializer::new(String::new());
	encode.append_pair("type", "rich");
	encode.append_pair("version", "1");
	encode.append_pair("provider_name", &site_name);
	encode.append_pair("provider_url", "https://www.fimfiction.net/");
	encode.append_pair("title", &group.name);
	if let Some(founder_link) = founder_link {
		encode.append_pair("author_name", &founder_link);
		encode.append_pair("author_url", &founder.link);
	}
	encode.append_pair("cache_age", "86400");
	encode.append_pair("html", "");
	let encode = encode.finish();
	text.push_str(&format!(
		r#"<link rel="alternate" type="application/json+oembed" href="https://www.fixfiction.net/oembed?{encode}" title="{}" />"#,
		&group.name));
	text.push_str(r#"</head><body></body></html>"#);
	text
}
