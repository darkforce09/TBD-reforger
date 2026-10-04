//! The shape a post is edited as, and the conversions between it and the wire.
//!
//! **Role:** the editor's own record of one post, the routes the screen calls, and the mapping in
//! both directions between a post's category and the tag the API stores it under.
//! **Position:** shared by the list, the editor form and the publish path; no view, no request.
//! **Signals & state:** none — a post is plain owned data.
//! **Invariants:** the tag set the API accepts is smaller than the set of categories this screen
//! offers, so standing orders are stored under the closest tag rather than refused — publishing one
//! reaches the real endpoint instead of pretending locally. That mapping is not reversible, so a
//! post read back from the wire under that tag comes back as an announcement. A post that has been
//! saved carries a server-minted identifier, which is how the publish path knows whether to create
//! or to update.

#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::Announcement;

/// One post, as the editor holds it.
///
/// `id` is either a server-minted identifier or a local one for a post that has never been saved,
/// `title` and `body` are what is being written, `category` is the label this screen offers,
/// `published` says whether it is live, `date` is the day shown in the list, and `thumbnail_url`
/// the hero image the publish payload carries.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, PartialEq)]
pub(super) struct Doc {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) category: String,
    pub(super) published: bool,
    pub(super) date: String,
    pub(super) body: String,
    /// The hero image, set by the upload control and read back from the listing.
    pub(super) thumbnail_url: String,
}

/// The categories the editor offers, as the stored value paired with its label.
#[cfg(target_arch = "wasm32")]
pub(super) const CATEGORY_OPTIONS: &[(&str, &str)] = &[
    ("announcement", "Announcement"),
    ("sop", "SOP"),
    ("event", "Community Event"),
    ("modpack", "Modpack Update"),
    ("important", "Important"),
];

/// The admin listing route: drafts and published posts together.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn announcement_list_path() -> &'static str {
    "/cms/announcements?limit=100"
}

/// The create route.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn announcement_create_path() -> &'static str {
    "/cms/announcements"
}

/// The edit and archive route for one post.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn announcement_id_path(id: &str) -> String {
    format!("/cms/announcements/{id}")
}

/// The route that pushes one post to the chat server again.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn announcement_push_path(id: &str) -> String {
    format!("/cms/announcements/{id}/push-discord")
}

/// Whether an identifier was minted by the server rather than made up locally for a new post.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn is_server_id(id: &str) -> bool {
    let b = id.as_bytes();
    b.len() == 36 && b[8] == b'-' && b[13] == b'-' && b[18] == b'-' && b[23] == b'-'
}

/// The tag a category is stored under, or nothing when the category is unknown.
///
/// The API's tag set has no entry for a standing order, so one is stored under the closest tag
/// that does exist. Publishing therefore reaches the real endpoint rather than reporting a local
/// success over a request that was never made.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn category_tag(category: &str) -> Option<&'static str> {
    match category {
        "announcement" | "sop" => Some("update"),
        "event" => Some("event"),
        "modpack" => Some("modpack_update"),
        "important" => Some("important"),
        _ => None,
    }
}

/// The category a stored tag reads back as.
///
/// Not an exact inverse: a standing order and an announcement share one tag on the wire, so both
/// come back as an announcement.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn tag_category(tag: &str) -> String {
    match tag {
        "event" => "event".into(),
        "modpack_update" => "modpack".into(),
        "important" => "important".into(),
        _ => "announcement".into(),
    }
}

/// The date part of a timestamp, or an empty string when there is not one.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn date_ymd(iso: &str) -> String {
    if iso.len() >= 10 {
        iso[..10].to_string()
    } else {
        String::new()
    }
}

/// One listed announcement as a post, or nothing when the row carries no identifier.
///
/// The date shown is the most specific one the row has: when it was published, else when it was
/// last changed, else when it was created.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn doc_from_announcement(v: &Announcement) -> Option<Doc> {
    if v.id.as_str().is_empty() {
        return None;
    }
    let published = v.status == "published";
    let published_at = v.published_at.as_deref().unwrap_or_default();
    let date_src = if !published_at.is_empty() {
        published_at
    } else if !v.updated_at.is_empty() {
        v.updated_at.as_str()
    } else {
        v.created_at.as_str()
    };
    Some(Doc {
        id: v.id.to_string(),
        title: v.title.clone(),
        category: tag_category(&v.tag),
        published,
        date: date_ymd(date_src),
        body: v.body.clone(),
        thumbnail_url: v.thumbnail_url.clone(),
    })
}
