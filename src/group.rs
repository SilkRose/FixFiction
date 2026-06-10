//! Request a [Group] and to format it in HTML.

use crate::database::{get_group, insert_group, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::group::GroupApi;
use crate::html_template::embed_html_template;
use crate::structs::{AppState, Color, Cover, EmbedData, Group, Parameters};
use crate::user::{User, request_user};
use crate::utility::{
	get_color, map_picture, parse_fimfic_response, unsupported_color_opt, unsupported_cover_opt,
};
use crate::{check_recache, get_variant};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};
use std::error::Error;

/// Requests a [Group] from the cache. If it's not cached, it will be requested from Fimfiction.net (and also cached).
///
/// #### Errors
/// This function will return an error in the following cases:
/// - Can't connect to the database
/// - If the group is uncached:
///   - Can't connect to Fimfiction
///   - Can't deserialize response from Fimfiction
pub(crate) async fn request_group(
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

/// Formats a [Group] to an HTML string for embedding. All groups have a founding [User].
///
/// #### Panics
///
/// Panics if stats are requested and the [Group]'s stats can't be formatted.
pub(crate) fn group_html_template(
	group: Group, founder: User, parameters: Parameters, link: String, errors: Vec<String>,
) -> String {
	let mut errors = errors;
	let founder_name = match parameters.stats {
		true => Some(format!("Founder: {}", founder.name)),
		false => None,
	};
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::Founder => Some(founder.color_hex),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(group.id))),
			_ => unsupported_color_opt(&mut errors, color.to_string(), Some(founder.color_hex)),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => Some(get_color(Some(group.id))),
				Cover::User | Cover::Founder => Some(founder.color_hex),
				Cover::None => None,
			},
			None => Some(get_color(Some(group.id))),
		},
	};
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::Founder => map_picture(founder.profile_pic_url),
			Cover::None => None,
			_ => unsupported_cover_opt(
				&mut errors,
				cover.to_string(),
				map_picture(group.icon_url).or(map_picture(founder.profile_pic_url)),
			),
		},
		None => map_picture(group.icon_url).or(map_picture(founder.profile_pic_url)),
	};
	let site_name = if parameters.stats {
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
			format_number_unit_metric(group.members as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(group.stories as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
		)
	} else {
		"Fimfiction".to_string()
	};
	let data = EmbedData {
		title: group.name,
		description: group.description,
		link,
		color,
		cover,
		site_name,
		site_url: String::from("https://www.fimfiction.net/"),
		errors: errors.to_vec(),
		user_name: founder_name,
		user_link: Some(founder.link),
		html_comment: None,
		open_graph_type: String::from("profile"),
		open_graph_property: Some(String::from("profile:username")),
	};
	embed_html_template(data)
}
