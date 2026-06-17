//! Request a group [Thread] and to format it in HTML.

use crate::database::{get_thread, insert_group, insert_thread, insert_user};
use crate::error::{Error, Result};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::thread::{ThreadApi, ThreadData};
use crate::group::{Group, request_group};
use crate::html_template::{EmbedData, embed_html_template};
use crate::parameters::{Color, Cover, Parameters};
use crate::user::{User, request_user};
use crate::utility::{
	get_color, map_picture, parse_date, parse_fimfic_response, unsupported_color_opt,
	unsupported_cover_opt,
};
use crate::{check_recache, get_variant, get_variants};
use chrono::{DateTime, TimeDelta, Utc};
use pony::http::Request;
use pony::number_format::{FormatType, format_number_unit_metric};
use sqlx::{Pool, Postgres};

/// Fimfiction thread data converted into a more usable structure
#[derive(Debug, Clone)]
pub(crate) struct Thread {
	pub(crate) id: i32,
	pub(crate) group_id: i32,
	pub(crate) creator_id: i32,
	pub(crate) last_poster_id: i32,
	pub(crate) title: String,
	pub(crate) link: String,
	pub(crate) posts: i32,
	pub(crate) sticky: bool,
	pub(crate) locked: bool,
	pub(crate) date_created: DateTime<Utc>,
	pub(crate) date_last_post: DateTime<Utc>,
	pub(crate) date_cached: DateTime<Utc>,
}

impl TryFrom<ThreadData<i32>> for Thread {
	type Error = Error;
	/// Converts Fimfiction's API response [ThreadData] into a [Thread]
	fn try_from(value: ThreadData<i32>) -> Result<Self> {
		let thread = Thread {
			id: value.id.parse()?,
			group_id: value.relationships.group.data.id.parse()?,
			creator_id: value.relationships.creator.data.id.parse()?,
			last_poster_id: value.relationships.last_poster.data.id.parse()?,
			title: value.attributes.title,
			link: value.meta.url,
			posts: value.attributes.num_posts,
			sticky: value.attributes.sticky,
			locked: value.attributes.locked,
			date_created: parse_date(value.attributes.date_created, "created")?.into(),
			date_last_post: parse_date(value.attributes.date_last_post, "last post")?.into(),
			date_cached: Utc::now(),
		};
		Ok(thread)
	}
}

/// [Thread] data combined with the last poster and creator [User] data
#[derive(Debug, Clone)]
pub(crate) struct ThreadReturn {
	pub(crate) thread: Thread,
	pub(crate) creator: User,
	pub(crate) last_poster: User,
}

/// Requests a [Thread] from the cache. If it's not cached, it will be requested from Fimfiction.net (and also cached).
///
/// #### Errors
/// This function will return an error in the following cases:
/// - Can't connect to the database
/// - If the thread is uncached:
///   - Can't connect to Fimfiction
///   - Can't deserialize response from Fimfiction
pub(crate) async fn request_thread(
	group_id: i32, thread_id: i32, api: &Request, db: &Pool<Postgres>, recache: bool,
) -> Result<(Group, User, Option<ThreadReturn>)> {
	let thread = get_thread(thread_id, db).await?;
	let thread = check_recache!(thread, recache, app);
	match thread {
		Some(thread) => {
			let (group, founder) = request_group(group_id, api, db, recache).await?;
			if thread.group_id != group.id {
				return Err("FixFiction error: group ID does not match with thread".into());
			}
			let data = build_thread_return(thread, api, db, recache).await?;
			Ok((group, founder, Some(data)))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/groups/{group_id}/threads?include=group,group.founder,creator,last_poster&page[size]=100"
			);
			let res = parse_fimfic_response::<ThreadApi<i32>>(api, &fimfic).await?;
			let group = get_variant!(res.included, ApiIncluded::Group)
				.ok_or("Fimfiction API error: no group included")?;
			let mut users = Vec::new();
			for user in get_variants!(res.included, ApiIncluded::Author) {
				let user = User::try_from(user.clone())?;
				insert_user(&user, db).await?;
				users.push(user);
			}
			let group = Group::try_from(group.clone())?;
			insert_group(&group, db).await?;
			let mut threads = Vec::new();
			for thread in res.data {
				let thread = Thread::try_from(thread)?;
				insert_thread(&thread, db).await?;
				threads.push(thread);
			}
			let founder = users
				.into_iter()
				.find(|user| user.id == group.founder_id)
				.ok_or("Fimfiction API error: no group founder included")?;
			let thread = threads.into_iter().find(|thread| thread.id == thread_id);
			if let Some(thread) = thread {
				let data = build_thread_return(thread, api, db, recache).await?;
				Ok((group, founder, Some(data)))
			} else {
				Ok((group, founder, None))
			}
		}
	}
}

/// Collects the [User]s who created and last posted in a [Thread] into a common struct.
///
/// #### Errors
/// This function will return an error in the following cases:
/// - Can't connect to the database
/// - If the thread is uncached:
///   - Can't connect to Fimfiction
///   - Can't deserialize response from Fimfiction
async fn build_thread_return(
	thread: Thread, api: &Request, db: &Pool<Postgres>, recache: bool,
) -> Result<ThreadReturn> {
	let creator = request_user(thread.creator_id, api, db, recache).await?;
	let last_poster = request_user(thread.last_poster_id, api, db, recache).await?;
	Ok(ThreadReturn {
		thread,
		creator,
		last_poster,
	})
}

/// Formats a group [Thread] to an HTML string for embedding. All threads are contained in [Group]s, which are founded by [User]s.
///
/// #### Panics
///
/// Panics if stats are requested and the [Thread]'s stats can't be formatted.
pub(crate) fn thread_html_template(
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
