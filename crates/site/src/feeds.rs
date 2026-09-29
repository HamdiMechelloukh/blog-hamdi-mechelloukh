//! Veille : récupération des flux RSS au build, via rss2json (comme l'ancien hook useRssFeeds).

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::content::Feed;

const RSS2JSON: &str = "https://api.rss2json.com/v1/api.json";
const ARTICLES_PER_FEED: usize = 2;
const SUMMARY_CHARS: usize = 160;

pub struct WatchItem {
    pub title: String,
    pub link: String,
    /// "28/09/2026" : même rendu en fr-FR et en-GB.
    pub date: String,
    pub description: String,
    pub thumbnail: Option<String>,
    pub source: String,
}

#[derive(Deserialize)]
struct Rss2JsonResponse {
    status: String,
    message: Option<String>,
    #[serde(default)]
    items: Vec<Rss2JsonItem>,
}

#[derive(Deserialize)]
struct Rss2JsonItem {
    title: String,
    link: String,
    #[serde(rename = "pubDate")]
    pub_date: String,
    #[serde(default)]
    description: String,
    thumbnail: Option<String>,
}

/// Un flux en erreur est journalisé et ignoré : la veille ne doit pas bloquer le build.
pub fn fetch_all(feeds: &[Feed]) -> Vec<WatchItem> {
    let client = reqwest::blocking::Client::new();
    let mut items = Vec::new();
    for feed in feeds {
        match fetch_feed(&client, feed) {
            Ok(feed_items) => items.extend(feed_items),
            Err(error) => eprintln!("veille : flux {} ignoré : {error:#}", feed.name),
        }
    }
    items
}

fn fetch_feed(client: &reqwest::blocking::Client, feed: &Feed) -> Result<Vec<WatchItem>> {
    let response: Rss2JsonResponse = client
        .get(RSS2JSON)
        .query(&[("rss_url", &feed.url)])
        .send()?
        .error_for_status()?
        .json()
        .context("réponse rss2json illisible")?;
    if response.status != "ok" {
        bail!(response.message.unwrap_or_else(|| "Feed error".to_string()));
    }
    Ok(response
        .items
        .into_iter()
        .take(ARTICLES_PER_FEED)
        .map(|item| WatchItem {
            date: day_month_year(&item.pub_date),
            description: summarize(&item.description),
            thumbnail: item.thumbnail.filter(|url| !url.is_empty()),
            title: item.title,
            link: item.link,
            source: feed.name.clone(),
        })
        .collect())
}

/// "2026-09-28 10:00:00" -> "28/09/2026".
fn day_month_year(pub_date: &str) -> String {
    match pub_date.get(..10).map(|date| date.split('-').collect::<Vec<_>>()).as_deref() {
        Some([year, month, day]) => format!("{day}/{month}/{year}"),
        _ => pub_date.to_string(),
    }
}

/// Retire les balises HTML et tronque à 160 caractères.
fn summarize(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }
    let mut text = String::new();
    let mut in_tag = false;
    for character in html.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    let truncated: String = text.chars().take(SUMMARY_CHARS).collect();
    format!("{}…", truncated.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn description_is_stripped_and_truncated() {
        assert_eq!(summarize("<p>Hello <b>world</b></p>"), "Hello world…");
        assert_eq!(summarize(&"é".repeat(300)).chars().count(), SUMMARY_CHARS + 1);
    }

    #[test]
    fn pub_date_is_reformatted() {
        assert_eq!(day_month_year("2026-09-28 10:00:00"), "28/09/2026");
    }
}
