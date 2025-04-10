use crate::database::{get_chapter, insert_chapter, insert_story, insert_user};
use crate::fimfiction_api::chapter::{ChapterApi, ChapterIncluded};
use crate::get_variant;
use crate::story::request_story;
use crate::structs::{AppState, Chapter, Story, User};
use crate::utility::parse_fimfic_response;
use chrono::{TimeDelta, Utc};

pub async fn request_chapter(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Chapter, Story, User), Box<dyn std::error::Error>> {
	let chapter = get_chapter(id, &app.db).await?;
	let chapter = match recache {
		true => chapter.filter(|chapter| {
			Utc::now()
				.checked_sub_signed(TimeDelta::seconds(app.cache_recache_age))
				.is_some_and(|max_age| chapter.date_cached >= max_age)
		}),
		false => chapter,
	};
	match chapter {
		Some(chapter) => {
			let (story, user) = request_story(chapter.story_id, app, recache).await?;
			Ok((chapter, story, user))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/chapters/{id}?include=story,story.author&fields%5Bchapter%5D=chapter_number,title,published,num_words,num_views,date_published,date_modified,authors_note,authors_note_position,content"
			);
			let api = parse_fimfic_response::<ChapterApi<i32>>(&app.api, &fimfic).await?;
			let story = get_variant!(api.included, ChapterIncluded::Story)
				.ok_or("Fimfiction API error: no story included")?;
			let author = get_variant!(api.included, ChapterIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let author = insert_user(None, author, &app.db).await?;
			let story = insert_story(None, story.clone(), author.id, &app.db).await?;
			let chapter = insert_chapter(Some(id), api.data, story.id, &app.db).await?;
			Ok((chapter, story, author))
		}
	}
}
