use crate::database::{get_chapter, get_story_chapter, insert_chapter, insert_story, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::chapter::ChapterApi;
use crate::fimfiction_api::story::StoryApi;
use crate::story::request_story;
use crate::structs::{AppState, Chapter, Story, User};
use crate::utility::parse_fimfic_response;
use crate::{check_recache, get_variant};
use chrono::{TimeDelta, Utc};

pub async fn request_chapter(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Chapter, Story, User), Box<dyn std::error::Error>> {
	let chapter = get_chapter(id, &app.db).await?;
	let chapter = check_recache!(chapter, recache, app);
	match chapter {
		Some(chapter) => {
			let (story, user) = request_story(chapter.story_id, app, recache).await?;
			Ok((chapter, story, user))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/chapters/{id}?include=story,story.author"
			);
			let api = parse_fimfic_response::<ChapterApi<i32>>(&app.api, &fimfic).await?;
			let story = get_variant!(api.included, ApiIncluded::Story)
				.ok_or("Fimfiction API error: no story included")?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let author = insert_user(None, author, &app.db).await?;
			let story = insert_story(None, story.clone(), author.id, &app.db).await?;
			let chapter = insert_chapter(Some(id), api.data, story.id, &app.db).await?;
			Ok((chapter, story, author))
		}
	}
}

pub async fn request_story_chapters(
	story_id: i32, chapter_num: i32, app: &AppState, recache: bool,
) -> Result<(Chapter, Story, User), Box<dyn std::error::Error>> {
	let chapter = get_story_chapter(story_id, chapter_num, &app.db).await?;
	let chapter = check_recache!(chapter, recache, app);
	match chapter {
		Some(chapter) => {
			let (story, user) = request_story(chapter.story_id, app, recache).await?;
			Ok((chapter, story, user))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/stories/{story_id}?include=author,chapters"
			);
			let api = parse_fimfic_response::<StoryApi<i32>>(&app.api, &fimfic).await?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let author = insert_user(None, author, &app.db).await?;
			let story = insert_story(None, api.data, author.id, &app.db).await?;
			let mut chapters = Vec::new();
			for chapter in &api.included {
				if let ApiIncluded::Chapter(data) = chapter {
					let inserted = insert_chapter(None, data.clone(), story_id, &app.db)
						.await
						.unwrap();
					chapters.push(inserted);
				}
			}
			let chapter = chapters
				.into_iter()
				.find(|chapter| chapter.chapter_num == chapter_num)
				.ok_or("FixFiction error: chapter not found")?;
			Ok((chapter, story, author))
		}
	}
}
