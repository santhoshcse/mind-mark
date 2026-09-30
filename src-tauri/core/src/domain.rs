use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    pub id: String,
    pub app_profile_id: String,
    pub folder_id: Option<String>,
    pub folder_name: Option<String>,
    pub position: i64,
    pub title: String,
    pub url: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub source_profile_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChromeProfile {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub profile_name: String,
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub folders: usize,
    pub skipped: usize,
    pub missing: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBookmarkInput {
    pub app_profile_id: String,
    pub folder_id: Option<String>,
    pub title: String,
    pub url: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBookmarkInput {
    pub id: String,
    pub title: String,
    pub url: String,
    pub folder_id: Option<String>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkFolder {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppProfile {
    pub id: String,
    pub name: String,
}
