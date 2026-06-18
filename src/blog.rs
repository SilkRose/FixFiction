//! Request a [Blog] and to format it in HTML.

use crate::database::Db;
use crate::error::{EmbedError, EmbedResult, Error, Result};
use crate::fimfiction_api::ApiIncluded;
use crate::fimfiction_api::blog::{BlogApi, BlogData};
use crate::html_template::{EmbedData, embed_html_template};
use crate::parameters::{Color, Cover, Parameters, parse_embed_parameters};
use crate::story::{Story, request_story};
use crate::user::{User, request_user};
use crate::utility::{
	clean_content, get_color, map_cover, map_picture, parse_fimfic_response, parse_id,
	trim_content, unsupported_color, unsupported_cover_opt,
};
use crate::{check_recache, get_variant};
use actix_web::web::{Path, Query, ThinData};
use actix_web::{HttpResponse, Responder, get};
use chrono::{DateTime, TimeDelta, Utc};
use pony::http::Request;
use pony::number_format::{FormatType, format_number_unit_metric};
use pony::word_stats::word_count;
use std::collections::HashMap;

/// Fimfiction blog data converted into a more usable structure
#[derive(Debug, Clone)]
pub(crate) struct Blog {
	pub(crate) id: i32,
	pub(crate) title: String,
	pub(crate) content: String,
	pub(crate) link: String,
	pub(crate) comments: i32,
	pub(crate) views: i32,
	pub(crate) author_id: i32,
	pub(crate) tags: String,
	pub(crate) story_id: Option<i32>,
	pub(crate) date_posted: DateTime<Utc>,
	pub(crate) date_cached: DateTime<Utc>,
}

impl TryFrom<BlogData<i32>> for Blog {
	type Error = Error;
	/// Converts Fimfiction's API response [BlogData] into a [Blog]
	fn try_from(value: BlogData<i32>) -> Result<Self> {
		let blog = Self {
			id: value.id.parse()?,
			title: value.attributes.title,
			content: value.attributes.content,
			link: value.meta.url,
			comments: value.attributes.num_comments,
			views: value.attributes.num_views,
			author_id: value.relationships.author.data.id.parse()?,
			tags: value.attributes.tags.join(", "),
			story_id: (value.relationships.tagged_story.data.id != "0")
				.then_some(value.relationships.tagged_story.data.id.parse::<i32>()?),
			date_posted: DateTime::parse_from_rfc3339(&value.attributes.date_posted)
				.map_err(|_| "FixFiction Error: failed to parse date posted")?
				.into(),
			date_cached: Utc::now(),
		};
		Ok(blog)
	}
}

/// The `blog/` endpoint.
///
/// Requests a blog by ID.
#[get("/blog/{id:.*}")]
async fn get_blog_endpoint(
	api: ThinData<Request>, db: ThinData<Db>, path: Path<String>,
	queries: Query<HashMap<String, String>>,
) -> EmbedResult<impl Responder> {
	let mut path = path.into_inner();
	let queries = queries.into_inner();
	let blog_id = parse_id(&path).map_embed_err("blog", &path)?;
	let (params, errors) = parse_embed_parameters(&mut path, queries, &db).await;
	let link = format!("https://www.fimfiction.net/blog/{path}");
	let (blog, user, story) = request_blog(blog_id, &api, &db, params.refresh)
		.await
		.map_embed_err("blog", &path)?;
	let body = blog_html_template(blog, user, story, params, link, errors);
	Ok(HttpResponse::Ok()
		.content_type("text/html; charset=utf-8")
		.body(body))
}

/// Requests a [Blog] from the cache. If it's not cached, it will be requested from Fimfiction.net (and also cached).
///
/// #### Errors
/// This function will return an error in the following cases:
/// - Can't connect to the database
/// - If the blog is uncached:
///   - Can't connect to Fimfiction
///   - Can't deserialize response from Fimfiction
pub(crate) async fn request_blog(
	id: i32, api: &Request, db: &Db, recache: bool,
) -> Result<(Blog, User, Option<Story>)> {
	let blog = db.get_blog(id).await?;
	let blog = check_recache!(blog, recache, app);
	match blog {
		Some(blog) => {
			let (story, user) = if let Some(story_id) = blog.story_id {
				let (story, user, _tags) = request_story(story_id, api, db, recache).await?;
				(Some(story), user)
			} else {
				(None, request_user(blog.author_id, api, db, recache).await?)
			};
			Ok((blog, user, story))
		}
		None => {
			let fimfic = format!(
				"https://www.fimfiction.net/api/v2/blog-posts/{id}?include=author&fields[blog_post]=title,date_posted,content,num_views,num_comments,site_post,tags,author,tagged_story"
			);
			let res = parse_fimfic_response::<BlogApi<i32>>(api, &fimfic).await?;
			let author = get_variant!(res.included, ApiIncluded::Author)
				.ok_or("Fimfiction API error: no author included")?;
			let story_id = (res.data.relationships.tagged_story.data.id != "0")
				.then_some(res.data.relationships.tagged_story.data.id.parse::<i32>()?);
			let (story, user) = if let Some(story_id) = story_id {
				let (story, user, _tags) = request_story(story_id, api, db, recache).await?;
				(Some(story), user)
			} else {
				let user = User::try_from(author.clone())?;
				db.insert_user(&user).await?;
				(None, user)
			};
			let blog = Blog::try_from(res.data)?;
			db.insert_blog(&blog).await?;
			Ok((blog, user, story))
		}
	}
}

/// Formats a [Blog] to an HTML string for embedding. Also requires the author (a [User]), and the blog's linked [Story] if present.
///
/// #### Panics
///
/// Panics if stats are requested and the [Blog]'s number of views or comments can't be formatted.
pub(crate) fn blog_html_template(
	blog: Blog, user: User, story: Option<Story>, parameters: Parameters, link: String,
	errors: Vec<String>,
) -> String {
	let mut errors = errors;
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
			Color::Story => match story {
				Some(ref story) => Some(story.color_hex.clone()),
				None => unsupported_color(&mut errors, color.to_string(), user.color_hex),
			},
			_ => unsupported_color(&mut errors, color.to_string(), user.color_hex),
		},
		None => match parameters.cover {
			Some(ref cover) => match cover {
				Cover::Story => match story {
					Some(ref story) => Some(story.color_hex.clone()),
					None => Some(user.color_hex),
				},
				Cover::User | Cover::Founder => Some(user.color_hex),
				Cover::None => None,
			},
			None => Some(user.color_hex),
		},
	};
	let cover = match parameters.cover {
		Some(cover) => match cover {
			Cover::User => map_picture(user.profile_pic_url),
			Cover::Story => match story {
				Some(ref story) => map_cover(story.cover_url.clone()),
				None => unsupported_cover_opt(
					&mut errors,
					cover.to_string(),
					map_picture(user.profile_pic_url),
				),
			},
			Cover::None => None,
			_ => unsupported_cover_opt(
				&mut errors,
				cover.to_string(),
				map_picture(user.profile_pic_url),
			),
		},
		None => map_picture(user.profile_pic_url),
	};
	let site_name = if parameters.stats {
		let time = blog.date_posted.format("%a %b %e %Y").to_string();
		format!(
			"Fimfiction - Posted: {time} 📅\nViews: {} 📈 Comments: {} 💬 Words: {} 📝",
			format_number_unit_metric(blog.views as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(blog.comments as f64, FormatType::MetricPrefix, 1, true)
				.unwrap(),
			format_number_unit_metric(
				word_count(&blog.content).unwrap() as f64,
				FormatType::MetricPrefix,
				1,
				true
			)
			.unwrap(),
		)
	} else {
		"Fimfiction".to_string()
	};
	let data = EmbedData {
		title: clean_content(blog.title),
		description: trim_content(blog.content, true),
		link,
		color,
		cover,
		site_name,
		site_url: String::from("https://www.fimfiction.net/"),
		errors: errors.to_vec(),
		user_name: Some(author),
		user_link: Some(user.link),
		html_comment: None,
		open_graph_type: String::from("article"),
		open_graph_property: Some(String::from("article:author")),
	};
	embed_html_template(data)
}
