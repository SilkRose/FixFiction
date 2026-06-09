//! Request a [Story] and to format it in HTML.

use crate::database::{
	get_story, get_tag, get_tag_links, insert_story, insert_tag, insert_tag_link, insert_user,
	remove_tag_links,
};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::story::StoryApi;
use crate::html_template::embed_html_template;
use crate::structs::{
	AppState, Color, CompletionStatus, ContentRating, Cover, EmbedData, Parameters, Story, Tag,
	User,
};
use crate::user::request_user;
use crate::utility::{
	get_color, map_cover, map_picture, map_tags, parse_fimfic_response, unsupported_color,
	unsupported_cover_opt,
};
use crate::{check_recache, get_variant, get_variants};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};

/// Requests a [Story] from the cache. If it's not cached, it will be requested from Fimfiction.net (and also cached).
///
/// #### Errors
/// This function will return an error in the following cases:
/// - Can't connect to the database
/// - If the story is uncached:
///   - Can't connect to Fimfiction
///   - Can't deserialize response from Fimfiction
pub async fn request_story(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Story, User, Vec<Tag>), Box<dyn std::error::Error>> {
	let story = get_story(id, &app.db).await?;
	let story = check_recache!(story, recache, app);
	match story {
		Some(story) => {
			let user = request_user(story.author_id, app, recache).await?;
			let tag_links = get_tag_links(story.id, &app.db).await?;
			let mut tags = Vec::with_capacity(tag_links.len());
			for link in tag_links {
				let tag = get_tag(link.tag_id, &app.db)
					.await?
					.expect("Database constraint means this will never fail.");
				tags.push(tag);
			}
			Ok((story, user, tags))
		}
		None => {
			let fimfic =
				format!("https://www.fimfiction.net/api/v2/stories/{id}?include=author,tags");
			let api = parse_fimfic_response::<StoryApi<i32>>(&app.api, &fimfic).await?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let api_tags = get_variants!(api.included, ApiIncluded::Tag).collect::<Vec<_>>();
			let user = insert_user(None, author, &app.db).await?;
			let story = insert_story(Some(id), api.data, user.id, &app.db).await?;
			remove_tag_links(id, &app.db).await?;
			let mut tags = Vec::with_capacity(api_tags.len());
			for tag in api_tags {
				let tag = insert_tag(None, tag.clone(), &app.db).await?;
				insert_tag_link(id, tag.id, &app.db).await?;
				tags.push(tag);
			}
			Ok((story, user, tags))
		}
	}
}

/// Formats a [Story] to an HTML string for embedding. All stories are authored by a [User].
///
/// #### Panics
///
/// Panics if stats are requested and the [Story]'s stats can't be formatted.
pub fn story_html_template(
	story: Story, user: User, mut tags: Vec<Tag>, parameters: Parameters, link: String,
	errors: Vec<String>,
) -> String {
	let mut errors = errors;
	let mut text = String::new();
	let author = if parameters.tags {
		tags.sort();
		format!("{}\nTags: {}", user.name, map_tags(&tags))
	} else {
		user.name
	};
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::User => Some(user.color_hex),
			Color::Story => Some(story.color_hex),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(story.id))),
			_ => unsupported_color(&mut errors, color.to_string(), story.color_hex),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => Some(story.color_hex),
				Cover::User => Some(user.color_hex),
				Cover::None => None,
				_ => Some(user.color_hex),
			},
			None => Some(story.color_hex),
		},
	};
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::Story => map_cover(story.cover_url),
			Cover::User => map_picture(user.profile_pic_url),
			Cover::None => None,
			_ => unsupported_cover_opt(&mut errors, cover.to_string(), map_cover(story.cover_url)),
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
				format_number_unit_metric(story.likes as f64, FormatType::MetricPrefix, 1, true)
					.unwrap(),
				format_number_unit_metric(story.dislikes as f64, FormatType::MetricPrefix, 1, true)
					.unwrap()
			)
		};
		format!(
			"Fimfiction - Published: {time} 📅 Status: {status}\nRating: {rating} {likes_dislikes}Views: {} 📈\nComments: {} 💬 Chapters: {} 📖 Words: {} 📝",
			format_number_unit_metric(story.views as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(story.comments as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(story.chapters as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(story.words as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
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
		errors: errors.to_vec(),
		user_name: Some(author),
		user_link: Some(user.link),
		html_comment: None,
		open_graph_type: String::from("book"),
		open_graph_property: Some(String::from("book:author")),
	};
	embed_html_template(data)
}
