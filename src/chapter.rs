use crate::database::{get_chapter, get_story_chapter, insert_chapter, insert_story, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::chapter::ChapterApi;
use crate::fimfiction_api::story::StoryApi;
use crate::html_template::embed_html_template;
use crate::story::request_story;
use crate::structs::{
	AppState, Chapter, Color, CompletionStatus, ContentRating, Cover, EmbedData, Parameters, Story,
	User,
};
use crate::utility::{get_color, map_cover, map_picture, map_tags, parse_fimfic_response};
use crate::{check_recache, get_variant, get_variants};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};

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
			let tags = get_variants!(api.included, ApiIncluded::Tag).collect::<Vec<_>>();
			let tags = map_tags(tags);
			let author = insert_user(None, author, &app.db).await?;
			let story = insert_story(None, story.clone(), &tags, author.id, &app.db).await?;
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
				"https://www.fimfiction.net/api/v2/stories/{story_id}?include=author,chapters,tags"
			);
			let api = parse_fimfic_response::<StoryApi<i32>>(&app.api, &fimfic).await?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let tags = get_variants!(api.included, ApiIncluded::Tag).collect::<Vec<_>>();
			let tags = map_tags(tags);
			let author = insert_user(None, author, &app.db).await?;
			let story = insert_story(None, api.data, &tags, author.id, &app.db).await?;
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

pub fn chapter_html_template(
	chapter: Chapter, story: Story, user: User, parameters: Parameters, link: String,
	errors: String,
) -> String {
	let author = match parameters.tags {
		true => format!("{} – {}\nTags: {}", user.name, story.title, story.tags),
		false => format!("{} – {}", user.name, story.title),
	};
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::User => Some(user.color_hex),
			Color::Story => Some(story.color_hex),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(chapter.id))),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => Some(story.color_hex),
				Cover::User => Some(user.color_hex),
				Cover::None => None,
			},
			None => Some(story.color_hex),
		},
	};
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::Story => map_cover(story.cover_url),
			Cover::User => map_picture(user.profile_pic_url),
			Cover::None => None,
		},
		None => map_cover(story.cover_url),
	};
	let site_name = if parameters.stats {
		let time = chapter.date_published.format("%a %b %e %Y").to_string();
		let status = match story.completion_status {
			CompletionStatus::Incomplete => "Incomplete 🔄",
			CompletionStatus::Complete => "Complete ✅",
			CompletionStatus::Hiatus => "Hiatus ⏸",
			CompletionStatus::Cancelled => "Cancelled ❌",
		};
		let rating = match story.content_rating {
			ContentRating::Everyone => "Everyone 🇪",
			ContentRating::Teen => "Teen 🇹",
			ContentRating::Mature => "Mature 🇲",
		};
		let likes_dislikes = if story.likes == -1 && story.dislikes == -1 {
			String::new()
		} else {
			format!(
				"Likes: {} 👍 Dislikes: {} 👎 ",
				format_number_unit_metric(story.likes as f64, FormatType::MetricPrefix, 1).unwrap(),
				format_number_unit_metric(story.dislikes as f64, FormatType::MetricPrefix, 1)
					.unwrap()
			)
		};
		format!(
			"Fimfiction - Published: {time} 📅 Status: {status}\nRating: {rating} {likes_dislikes}Views: {}/{} 📈\nComments: {} 💬 Chapter: {}/{} 📖 Words: {}/{} 📝",
			format_number_unit_metric(chapter.views as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(story.views as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(story.comments as f64, FormatType::MetricPrefix, 1).unwrap(),
			chapter.chapter_num,
			story.chapters,
			format_number_unit_metric(chapter.words as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(story.words as f64, FormatType::MetricPrefix, 1).unwrap(),
		)
	} else {
		"Fimfiction".to_string()
	};
	let data = EmbedData {
		title: chapter.title,
		description: story.short_description,
		link,
		color,
		cover,
		site_name,
		site_url: String::from("https://www.fimfiction.net/"),
		errors,
		user_name: Some(author),
		user_link: Some(user.link),
		html_comment: None,
		open_graph_type: String::from("book"),
		open_graph_property: Some(String::from("book:author")),
	};
	embed_html_template(data)
}
