use crate::database::{get_thread, insert_group, insert_thread, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::thread::ThreadApi;
use crate::group::request_group;
use crate::structs::{AppState, Group, Thread, ThreadReturn, User};
use crate::user::request_user;
use crate::utility::parse_fimfic_response;
use crate::{check_recache, get_variant, get_variants};
use chrono::{TimeDelta, Utc};

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
			let mut threads = Vec::new();
			for thread in api.data {
				let thread = insert_thread(None, thread, group_id, &app.db).await?;
				threads.push(thread);
			}
			let group = insert_group(Some(group_id), group, &app.db).await?;
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
