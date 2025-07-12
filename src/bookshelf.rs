use crate::database::{get_bookshelf, insert_bookshelf, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::bookshelf::BookshelfApi;
use crate::structs::{AppState, Bookshelf, User};
use crate::user::request_user;
use crate::utility::parse_fimfic_response;
use crate::{check_recache, get_variant};
use chrono::{TimeDelta, Utc};

pub async fn request_bookshelf(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Bookshelf, Option<User>), Box<dyn std::error::Error>> {
	let bookshelf = get_bookshelf(id, &app.db).await?;
	let bookshelf = check_recache!(bookshelf, recache, app);
	match bookshelf {
		Some(bookshelf) => {
			let (bookshelf, user) = if let Some(user_id) = bookshelf.user_id {
				let user = request_user(user_id, app, recache).await?;
				(bookshelf, Some(user))
			} else {
				(bookshelf, None)
			};
			Ok((bookshelf, user))
		}
		None => {
			let fimfic = format!("https://www.fimfiction.net/api/v2/bookshelves/{id}?include=user");
			let api = parse_fimfic_response::<BookshelfApi<i32>>(&app.api, &fimfic).await?;
			let user = get_variant!(api.included, ApiIncluded::Author);
			if let Some(user) = user {
				let user = insert_user(None, user, &app.db).await?;
				let bookshelf =
					insert_bookshelf(Some(id), &api.data, Some(user.id), &app.db).await?;
				Ok((bookshelf, Some(user)))
			} else {
				let bookshelf = insert_bookshelf(Some(id), &api.data, None, &app.db).await?;
				Ok((bookshelf, None))
			}
		}
	}
}
