use crate::database::{get_story, insert_story, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::story::StoryApi;
use crate::html_template::embed_html_template;
use crate::structs::{
	AppState, Color, CompletionStatus, ContentRating, Cover, EmbedData, Parameters, Story, User,
};
use crate::user::request_user;
use crate::utility::{get_color, map_cover, map_picture, map_tags, parse_fimfic_response};
use crate::{check_recache, get_variant, get_variants};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};

pub async fn request_story(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Story, User), Box<dyn std::error::Error>> {
	let story = get_story(id, &app.db).await?;
	let story = check_recache!(story, recache, app);
	match story {
		Some(story) => {
			let user = request_user(story.author_id, app, recache).await?;
			Ok((story, user))
		}
		None => {
			let fimfic =
				format!("https://www.fimfiction.net/api/v2/stories/{id}?include=author,tags");
			let api = parse_fimfic_response::<StoryApi<i32>>(&app.api, &fimfic).await?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let tags = get_variants!(api.included, ApiIncluded::Tag).collect::<Vec<_>>();
			let tags = map_tags(tags);
			let user = insert_user(None, author, &app.db).await?;
			let story = insert_story(Some(id), api.data, &tags, user.id, &app.db).await?;
			Ok((story, user))
		}
	}
}

pub fn story_html_template(
	story: Story, user: User, parameters: Parameters, link: String, errors: String,
) -> String {
	let mut text = String::new();
	let author = match parameters.tags {
		true => format!("{}\nTags: {}", user.name, story.tags),
		false => user.name,
	};
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::User => Some(user.color_hex),
			Color::Story => Some(story.color_hex),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(story.id))),
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
	if let Some(ref cover) = cover {
		text.push_str(&format!(
			r#"<meta property="og:image" content="{cover}" />"#
		));
	}
	let site_name = if parameters.stats {
		let time = story.date_published.format("%a %b %e %Y").to_string();
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
			"Fimfiction - Published: {time} 📅 Status: {status}\nRating: {rating} {likes_dislikes}Views: {} 📈\nComments: {} 💬 Chapters: {} 📖 Words: {} 📝",
			format_number_unit_metric(story.views as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(story.comments as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(story.chapters as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(story.words as f64, FormatType::MetricPrefix, 1).unwrap(),
		)
	} else {
		"Fimfiction".to_string()
	};
	let data = EmbedData {
		title: story.title,
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
