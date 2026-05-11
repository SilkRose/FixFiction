use crate::database::{get_thread, insert_group, insert_thread, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::thread::ThreadApi;
use crate::group::request_group;
use crate::html_template::embed_html_template;
use crate::structs::{
	AppState, Color, Cover, EmbedData, Group, Parameters, Thread, ThreadReturn, User,
};
use crate::user::request_user;
use crate::utility::{
	get_color, map_picture, parse_fimfic_response, unsupported_color_opt, unsupported_cover_opt,
};
use crate::{check_recache, get_variant, get_variants};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};

pub async fn request_thread(
	group_id: i32, thread_id: i32, app: &AppState, recache: bool,
) -> Result<(Group, User, Option<ThreadReturn>), Box<dyn std::error::Error>> {
	let thread = get_thread(thread_id, &app.db).await?;
	let thread = check_recache!(thread, recache, app);
	match thread {
		Some(thread) => {
			let (group, founder) = request_group(group_id, app, recache).await?;
			if thread.group_id != group.id {
				return Err("FixFiction error: group ID does not match with thread".into());
			}
			let data = build_thread_return(thread, app, recache).await?;
			Ok((group, founder, Some(data)))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/groups/{group_id}/threads?include=group,group.founder,creator,last_poster&page[size]=100"
			);
			let api = parse_fimfic_response::<ThreadApi<i32>>(&app.api, &fimfic).await?;
			let group = get_variant!(api.included, ApiIncluded::Group)
				.ok_or("Fimfiction API error: no group included")?;
			let mut users = Vec::new();
			for user in get_variants!(api.included, ApiIncluded::Author) {
				let user = insert_user(None, user, &app.db).await?;
				users.push(user);
			}
			let group = insert_group(Some(group_id), group, &app.db).await?;
			let mut threads = Vec::new();
			for thread in api.data {
				let thread = insert_thread(None, thread, group_id, &app.db).await?;
				threads.push(thread);
			}
			let founder = users
				.into_iter()
				.find(|user| user.id == group.founder_id)
				.ok_or("Fimfiction API error: no group founder included")?;
			let thread = threads.into_iter().find(|thread| thread.id == thread_id);
			if let Some(thread) = thread {
				let data = build_thread_return(thread, app, recache).await?;
				Ok((group, founder, Some(data)))
			} else {
				Ok((group, founder, None))
			}
		}
	}
}

async fn build_thread_return(
	thread: Thread, app: &AppState, recache: bool,
) -> Result<ThreadReturn, Box<dyn std::error::Error>> {
	let creator = request_user(thread.creator_id, app, recache).await?;
	let last_poster = request_user(thread.last_poster_id, app, recache).await?;
	Ok(ThreadReturn {
		thread,
		creator,
		last_poster,
	})
}

pub fn thread_html_template(
	group: Group, founder: User, thread_data: ThreadReturn, parameters: Parameters, link: String,
	errors: Vec<String>,
) -> String {
	let thread = thread_data.thread;
	let creator = thread_data.creator;
	let last_poster = thread_data.last_poster;
	let mut errors = errors;
	let stickied = match thread.sticky {
		true => "Pinned ✅",
		false => "Pinned ❌",
	};
	let locked = match thread.locked {
		true => "Locked 🔒",
		false => "Unlocked 🔓",
	};
	let desc = match parameters.stats {
		false => "Group thread post.".to_string(),
		true => format!(
			"Group thread post.\nCreated: {} 📅, Last post: {} 📅\nPosts: {} 🔢, {stickied}, {locked}\nCreator: {}, Last poster: {}",
			thread.date_created.format("%a %b %e %Y"),
			thread.date_last_post.format("%a %b %e %Y"),
			format_number_unit_metric(thread.posts as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			creator.name,
			last_poster.name
		),
	};
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::Founder => Some(founder.color_hex),
			Color::User => Some(creator.color_hex),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(group.id))),
			_ => unsupported_color_opt(&mut errors, color.to_string(), Some(founder.color_hex)),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => Some(get_color(Some(group.id))),
				Cover::Founder => Some(founder.color_hex),
				Cover::User => Some(creator.color_hex),
				Cover::None => None,
			},
			None => Some(get_color(Some(group.id))),
		},
	};
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::Founder => map_picture(founder.profile_pic_url),
			Cover::User => map_picture(creator.profile_pic_url),
			Cover::None => None,
			_ => unsupported_cover_opt(
				&mut errors,
				cover.to_string(),
				map_picture(group.icon_url).or(map_picture(founder.profile_pic_url)),
			),
		},
		None => map_picture(group.icon_url).or(map_picture(founder.profile_pic_url)),
	};
	let data = EmbedData {
		title: thread.title,
		description: desc,
		link,
		color,
		cover,
		site_name: String::from("Fimfiction"),
		site_url: String::from("https://www.fimfiction.net/"),
		errors: errors.to_vec(),
		user_name: Some(group.name),
		user_link: Some(group.link),
		html_comment: None,
		open_graph_type: String::from("profile"),
		open_graph_property: Some(String::from("profile:username")),
	};
	embed_html_template(data)
}
