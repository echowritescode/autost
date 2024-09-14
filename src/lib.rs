use std::{fs::File, io::Read, path::Path};

use askama::Template;
use jane_eyre::eyre::{self, OptionExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::meta::extract_metadata;

pub mod cohost;
pub mod dom;
pub mod meta;

#[derive(Clone, Debug, Default, PartialEq, Template)]
#[template(path = "post-meta.html")]
pub struct PostMeta {
    pub references: Vec<String>,
    pub title: Option<String>,
    pub published: Option<String>,
    pub author: Option<(String, String)>,
    pub tags: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct ExtractedPost {
    pub unsafe_html: String,
    pub meta: PostMeta,
}

#[derive(Clone, Debug, Template)]
#[template(path = "posts.html")]
pub struct PostsPageTemplate {
    pub post_groups: Vec<PostGroup>,
}

#[derive(Clone, Debug)]
pub struct PostGroup {
    pub posts: Vec<TemplatedPost>,
    pub meta: PostMeta,
}

#[derive(Clone, Debug)]
pub struct TemplatedPost {
    pub post_page_filename: Option<String>,
    pub post_page_href: Option<String>,
    pub meta: PostMeta,
    pub content: String,
}

impl TemplatedPost {
    pub fn load(path: &Path) -> eyre::Result<Self> {
        let mut file = File::open(&path)?;
        let mut unsafe_source = String::default();
        file.read_to_string(&mut unsafe_source)?;

        let unsafe_html = if path.ends_with(".md") {
            // author step: render markdown to html.
            render_markdown(&unsafe_source)
        } else {
            unsafe_source
        };

        // reader step: extract metadata.
        let post = extract_metadata(&unsafe_html)?;

        // reader step: filter html.
        let safe_html = ammonia::Builder::default()
            .add_generic_attributes(["style", "id"])
            .add_generic_attributes(["data-cohost-href", "data-cohost-src"]) // cohost2autost
            .add_tag_attributes("details", ["open"])
            .add_tag_attributes("img", ["loading"])
            .add_tags(["meta"])
            .add_tag_attributes("meta", ["name", "content"])
            .id_prefix(Some("user-content-")) // cohost compatibility
            .clean(&post.unsafe_html)
            .to_string();

        let original_name = path.file_name().ok_or_eyre("post has no file name")?;
        let original_name = original_name.to_str().ok_or_eyre("unsupported file name")?;
        let (post_page_filename, _) = original_name
            .rsplit_once(".")
            .unwrap_or((original_name, ""));
        let post_page_filename = format!("{post_page_filename}.html");

        Ok(TemplatedPost {
            post_page_filename: Some(post_page_filename.clone()),
            post_page_href: Some(post_page_filename.clone()),
            meta: post.meta,
            content: safe_html,
        })
    }
}

pub fn cli_init() -> eyre::Result<()> {
    jane_eyre::install()?;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    Ok(())
}

/// render markdown in a cohost-compatible way.
///
/// known discrepancies:
/// - `~~strikethrough~~` not handled
/// - @mentions not handled
/// - :emotes: not handled
/// - single newline always yields `<br>`
///   (this was not the case for older chosts, as reflected in their `.astMap`)
/// - blank lines in `<details>` close the element in some situations?
/// - spaced numbered lists yield separate `<ol start>` instead of `<li><p>`
pub fn render_markdown(markdown: &str) -> String {
    let mut options = comrak::Options::default();
    options.render.unsafe_ = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.render.hardbreaks = true;
    let unsafe_html = comrak::markdown_to_html(&markdown, &options);

    unsafe_html
}

#[test]
fn test_render_markdown() {
    assert_eq!(
        render_markdown("first\nsecond"),
        "<p>first<br />\nsecond</p>\n"
    );
}
