use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{
    domain::{ChromeProfile, SyncReport},
    error::AppError,
    Database,
};

#[derive(Debug, Clone)]
pub struct ParsedChromeBookmarks {
    pub folders: Vec<ParsedFolder>,
    pub bookmarks: Vec<ParsedBookmark>,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedFolder {
    pub source_folder_id: String,
    pub parent_source_folder_id: Option<String>,
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Clone)]
pub struct ParsedBookmark {
    pub source_bookmark_id: String,
    pub parent_source_folder_id: String,
    pub title: String,
    pub url: String,
    pub position: i64,
}

#[derive(Debug, Deserialize)]
struct ChromeBookmarkFile {
    #[serde(default)]
    roots: HashMap<String, ChromeNode>,
}

#[derive(Debug, Deserialize)]
struct ChromeNode {
    id: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(rename = "type", default)]
    node_type: String,
    url: Option<String>,
    #[serde(default)]
    children: Vec<ChromeNode>,
}

pub fn discover_chrome_profiles() -> Result<Vec<ChromeProfile>, AppError> {
    let local_app_data = env::var_os("LOCALAPPDATA").ok_or_else(|| {
        AppError::InvalidInput("Windows local app data directory is unavailable".into())
    })?;
    let root = PathBuf::from(local_app_data)
        .join("Google")
        .join("Chrome")
        .join("User Data");

    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let canonical_root = root.canonicalize()?;
    let mut profiles = Vec::new();
    for entry in fs::read_dir(&canonical_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        if name != "Default" && !name.starts_with("Profile ") {
            continue;
        }

        let path = entry.path().canonicalize()?;
        if !path.starts_with(&canonical_root) || !path.join("Bookmarks").is_file() {
            continue;
        }

        profiles.push(ChromeProfile {
            id: name.clone(),
            name,
            path: path.to_string_lossy().into_owned(),
        });
    }

    profiles.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(profiles)
}

pub fn parse_chrome_bookmarks(json: &str) -> Result<ParsedChromeBookmarks, AppError> {
    let file: ChromeBookmarkFile = serde_json::from_str(json)?;
    let mut parsed = ParsedChromeBookmarks {
        folders: Vec::new(),
        bookmarks: Vec::new(),
        skipped: 0,
        warnings: Vec::new(),
    };

    let preferred_roots = ["bookmark_bar", "other", "synced", "mobile"];
    let mut root_names: Vec<String> = preferred_roots
        .iter()
        .filter(|name| file.roots.contains_key(**name))
        .map(|name| (*name).to_string())
        .collect();
    let preferred: HashSet<&str> = preferred_roots.into_iter().collect();
    let mut remaining: Vec<String> = file
        .roots
        .keys()
        .filter(|name| !preferred.contains(name.as_str()))
        .cloned()
        .collect();
    remaining.sort();
    root_names.extend(remaining);

    for root_name in root_names {
        let Some(root) = file.roots.get(&root_name) else {
            continue;
        };
        let folder_id = root.id.clone().unwrap_or_else(|| root_name.clone());
        parsed.folders.push(ParsedFolder {
            source_folder_id: folder_id.clone(),
            parent_source_folder_id: None,
            name: if root.name.is_empty() {
                root_name.clone()
            } else {
                root.name.clone()
            },
            position: parsed.folders.len() as i64,
        });
        parse_children(&root.children, &folder_id, &mut parsed);
    }

    if parsed.skipped > 0 {
        parsed.warnings.push(format!(
            "Skipped {} unsupported or incomplete Chrome bookmark entries.",
            parsed.skipped
        ));
    }

    Ok(parsed)
}

fn parse_children(children: &[ChromeNode], parent_id: &str, parsed: &mut ParsedChromeBookmarks) {
    for (position, node) in children.iter().enumerate() {
        let Some(source_id) = node.id.as_ref().filter(|id| !id.is_empty()) else {
            parsed.skipped += 1;
            continue;
        };

        match node.node_type.as_str() {
            "folder" => {
                parsed.folders.push(ParsedFolder {
                    source_folder_id: source_id.clone(),
                    parent_source_folder_id: Some(parent_id.to_string()),
                    name: node.name.clone(),
                    position: position as i64,
                });
                parse_children(&node.children, source_id, parsed);
            }
            "url" => {
                let Some(url) = node.url.as_ref().filter(|url| !url.trim().is_empty()) else {
                    parsed.skipped += 1;
                    continue;
                };
                parsed.bookmarks.push(ParsedBookmark {
                    source_bookmark_id: source_id.clone(),
                    parent_source_folder_id: parent_id.to_string(),
                    title: node.name.clone(),
                    url: url.clone(),
                    position: position as i64,
                });
            }
            _ => parsed.skipped += 1,
        }
    }
}

pub fn sync_chrome_profile(database: &Database, profile_id: &str) -> Result<SyncReport, AppError> {
    let profile = discover_chrome_profiles()?
        .into_iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| AppError::InvalidInput("Chrome profile is no longer available".into()))?;
    let bookmarks_path = Path::new(&profile.path).join("Bookmarks");
    let contents = fs::read_to_string(bookmarks_path)?;
    let parsed = parse_chrome_bookmarks(&contents)?;
    database.sync_chrome_bookmarks(&profile, &parsed)
}

#[cfg(test)]
mod tests {
    use super::parse_chrome_bookmarks;

    #[test]
    fn parses_nested_folders_and_preserves_sibling_order() {
        let parsed = parse_chrome_bookmarks(
            r#"{
                "roots": {
                    "bookmark_bar": {
                        "id": "1", "name": "Bookmarks bar", "type": "folder",
                        "children": [
                            {"id": "2", "name": "Work", "type": "folder", "children": [
                                {"id": "3", "name": "Issue tracker", "type": "url", "url": "https://issues.example.com/a"},
                                {"id": "4", "name": "Docs", "type": "url", "url": "https://docs.example.com"}
                            ]},
                            {"id": "5", "name": "Home", "type": "url", "url": "https://home.example.com"}
                        ]
                    }
                }
            }"#,
        )
        .expect("fixture parses");

        assert_eq!(parsed.folders.len(), 2);
        assert_eq!(parsed.bookmarks.len(), 3);
        assert_eq!(parsed.bookmarks[0].parent_source_folder_id, "2");
        assert_eq!(parsed.bookmarks[1].position, 1);
        assert_eq!(parsed.bookmarks[2].parent_source_folder_id, "1");
    }

    #[test]
    fn reports_incomplete_entries_without_failing_valid_entries() {
        let parsed = parse_chrome_bookmarks(
            r#"{"roots":{"other":{"name":"Other","children":[
                {"id":"ok","name":"Valid","type":"url","url":"https://example.com"},
                {"id":"bad","name":"Missing URL","type":"url"},
                {"name":"Missing ID","type":"url","url":"https://ignored.example"},
                {"id":"separator","type":"separator"}
            ]}}}"#,
        )
        .expect("valid JSON parses");

        assert_eq!(parsed.bookmarks.len(), 1);
        assert_eq!(parsed.skipped, 3);
        assert_eq!(parsed.warnings.len(), 1);
    }
}
