// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/wiki-page.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::WikiArticle;

///The doctrine wiki: page summaries, the article with the typed blocks the server parses from its markdown, the administrator's save, and the revision history. The root is one article.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct WikiPageContract(pub WikiArticle);
impl ::std::ops::Deref for WikiPageContract {
    type Target = WikiArticle;
    fn deref(&self) -> &WikiArticle {
        &self.0
    }
}
impl ::std::convert::From<WikiArticle> for WikiPageContract {
    fn from(value: WikiArticle) -> Self {
        Self(value)
    }
}
