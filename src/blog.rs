use crate::database::{get_blog, insert_blog, insert_user};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::blog::BlogApi;
use crate::html_template::embed_html_template;
use crate::story::request_story;
use crate::structs::{AppState, Blog, Color, Cover, EmbedData, Parameters, Story, User};
use crate::user::request_user;
use crate::utility::{get_color, map_cover, map_picture, parse_fimfic_response};
use crate::{check_recache, get_variant};
use chrono::{TimeDelta, Utc};
use pony::number_format::{FormatType, format_number_unit_metric};

pub async fn request_blog(
	id: i32, app: &AppState, recache: bool,
) -> Result<(Blog, User, Option<Story>), Box<dyn std::error::Error>> {
	let blog = get_blog(id, &app.db).await?;
	let blog = check_recache!(blog, recache, app);
	match blog {
		Some(blog) => {
			let (story, user) = if let Some(story_id) = blog.story_id {
				let (story, user) = request_story(story_id, app, recache).await?;
				(Some(story), user)
			} else {
				(None, request_user(blog.author_id, app, recache).await?)
			};
			Ok((blog, user, story))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/blog-posts/{id}?include=author&fields[blog_post]=title,date_posted,content,num_views,num_comments,site_post,tags,tagged_story"
			);
			let api = parse_fimfic_response::<BlogApi<i32>>(&app.api, &fimfic).await?;
			let author = get_variant!(api.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let story_id = (api.data.relationships.tagged_story.data.id != "0")
				.then_some(api.data.relationships.tagged_story.data.id.parse::<i32>()?);
			let (story, user) = if let Some(story_id) = story_id {
				let (story, user) = request_story(story_id, app, recache).await?;
				(Some(story), user)
			} else {
				(None, insert_user(None, author, &app.db).await?)
			};
			let blog = insert_blog(Some(id), &api.data, user.id, story_id, &app.db).await?;
			Ok((blog, user, story))
		}
	}
}

pub fn blog_html_template(
	blog: Blog, user: User, story: Option<Story>, parameters: Parameters, link: String,
	errors: String,
) -> String {
	let author = match parameters.tags && !blog.tags.is_empty() {
		true => format!("{}\nTags: {}", user.name, blog.tags),
		false => user.name,
	};
	let color = match parameters.color {
		Some(color) => match color {
			Color::None => None,
			Color::Custom(color) => Some(color),
			Color::User => Some(user.color_hex),
			Color::Random => Some(get_color(None)),
			Color::Modulo => Some(get_color(Some(blog.id))),
			Color::Story => Some(
				story
					.clone()
					.map(|story| story.color_hex)
					.unwrap_or(user.color_hex),
			),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => match story {
					Some(ref story) => Some(story.color_hex.clone()),
					None => Some(user.color_hex),
				},
				Cover::User => Some(user.color_hex),
				Cover::None => None,
			},
			None => Some(user.color_hex),
		},
	};
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::User => map_picture(user.profile_pic_url),
			Cover::Story => story
				.map(|story| map_cover(story.cover_url))
				.unwrap_or(map_picture(user.profile_pic_url)),
			Cover::None => None,
		},
		None => map_picture(user.profile_pic_url),
	};
	let site_name = if parameters.stats {
		let time = blog.date_posted.format("%a %b %e %Y").to_string();
		format!(
			"Fimfiction - Posted: {time} 📅\nViews: {} 📈 Comments: {} 💬",
			format_number_unit_metric(blog.views as f64, FormatType::MetricPrefix, 1).unwrap(),
			format_number_unit_metric(blog.comments as f64, FormatType::MetricPrefix, 1).unwrap(),
		)
	} else {
		"Fimfiction".to_string()
	};
	let data = EmbedData {
		title: blog.title,
		description: blog.content,
		link,
		color,
		cover,
		site_name,
		site_url: String::from("https://www.fimfiction.net/"),
		errors,
		user_name: Some(author),
		user_link: Some(user.link),
		html_comment: None,
		open_graph_type: String::from("article"),
		open_graph_property: Some(String::from("article:author")),
	};
	embed_html_template(data)
}
