//! Publication sur chaque plateforme.

use anyhow::{Context, Result, bail};
use reqwest::blocking::{Client, Response};
use serde::Deserialize;
use serde_json::json;

use crate::QueueItem;
use crate::require_env;
use crate::state::{Platform, PostRecord};

pub fn publish(item: &QueueItem, canonical_url: &str) -> Result<PostRecord> {
    let client = Client::new();
    match item.platform {
        Platform::Devto => post_to_devto(&client, item, canonical_url),
        Platform::Linkedin => post_to_linkedin(&client, item, canonical_url),
        Platform::Medium => notify_medium_via_telegram(&client, item, canonical_url),
    }
}

/// Échec HTTP : le message d'erreur inclut le corps de la réponse, comme la version TypeScript.
fn check(response: Response, service: &str) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    bail!("{service} {}: {}", status.as_u16(), response.text().unwrap_or_default());
}

fn post_to_devto(client: &Client, item: &QueueItem, canonical_url: &str) -> Result<PostRecord> {
    #[derive(Deserialize)]
    struct Created {
        id: u64,
        url: String,
    }
    let meta = &item.article.meta;
    let tags: Vec<String> = meta
        .tags
        .iter()
        .take(4)
        .map(|tag| tag.to_lowercase().chars().filter(char::is_ascii_alphanumeric).collect::<String>())
        .filter(|tag| !tag.is_empty())
        .collect();
    let response = client
        .post("https://dev.to/api/articles")
        .header("api-key", require_env("DEVTO_API_KEY")?)
        .json(&json!({
            "article": {
                "title": meta.title,
                "body_markdown": item.article.markdown,
                "published": true,
                "canonical_url": canonical_url,
                "tags": tags,
                "description": meta.summary,
            }
        }))
        .send()?;
    let created: Created = check(response, "dev.to")?.json()?;
    Ok(PostRecord { url: Some(created.url), id: Some(created.id.to_string()), ..PostRecord::now() })
}

/// Access token LinkedIn. Avec un refresh token (LINKEDIN_REFRESH_TOKEN + LINKEDIN_CLIENT_ID +
/// LINKEDIN_CLIENT_SECRET), on l'échange à chaud contre un access token frais : le refresh token vit ~1 an,
/// l'access token ~60 j. Sinon on retombe sur LINKEDIN_ACCESS_TOKEN statique.
fn linkedin_access_token(client: &Client) -> Result<String> {
    #[derive(Deserialize)]
    struct Token {
        access_token: String,
    }
    let (Ok(refresh_token), Ok(client_id), Ok(client_secret)) = (
        require_env("LINKEDIN_REFRESH_TOKEN"),
        require_env("LINKEDIN_CLIENT_ID"),
        require_env("LINKEDIN_CLIENT_SECRET"),
    ) else {
        return require_env("LINKEDIN_ACCESS_TOKEN");
    };
    let response = client
        .post("https://www.linkedin.com/oauth/v2/accessToken")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
        ])
        .send()?;
    let token: Token = check(response, "LinkedIn refresh")?.json()?;
    Ok(token.access_token)
}

fn post_to_linkedin(client: &Client, item: &QueueItem, canonical_url: &str) -> Result<PostRecord> {
    let meta = &item.article.meta;
    let access_token = linkedin_access_token(client)?;
    let hashtags: Vec<String> = meta
        .tags
        .iter()
        .take(5)
        .map(|tag| format!("#{}", tag.chars().filter(char::is_ascii_alphanumeric).collect::<String>()))
        .filter(|hashtag| hashtag.len() > 1)
        .collect();
    let mut text = format!("{}\n\n{}\n\n🔗 {canonical_url}", meta.title, meta.summary);
    if !hashtags.is_empty() {
        text.push_str(&format!("\n\n{}", hashtags.join(" ")));
    }
    let response = client
        .post("https://api.linkedin.com/v2/ugcPosts")
        .bearer_auth(access_token)
        .header("X-Restli-Protocol-Version", "2.0.0")
        .json(&json!({
            "author": require_env("LINKEDIN_USER_URN")?,
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": { "text": text },
                    "shareMediaCategory": "ARTICLE",
                    "media": [{
                        "status": "READY",
                        "originalUrl": canonical_url,
                        "title": { "text": meta.title },
                        "description": { "text": meta.summary },
                    }],
                },
            },
            "visibility": { "com.linkedin.ugc.MemberNetworkVisibility": "PUBLIC" },
        }))
        .send()?;
    let response = check(response, "LinkedIn")?;
    let id = response.headers().get("x-restli-id").and_then(|value| value.to_str().ok()).map(str::to_string);
    Ok(PostRecord { id, ..PostRecord::now() })
}

fn notify_medium_via_telegram(client: &Client, item: &QueueItem, canonical_url: &str) -> Result<PostRecord> {
    let meta = &item.article.meta;
    let bot_token = require_env("TELEGRAM_BOT_TOKEN")?;
    let tag_list = meta.tags.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    let mark_done = format!("cargo run -p crossposter -- --mark-done medium {}", item.article.slug);
    let text = format!(
        "📝 <b>Medium import à faire</b>\n\n<b>{}</b>\n\n{}\n\n\
         1. <a href=\"https://medium.com/p/import\">medium.com/p/import</a>\n\
         2. Colle : <code>{}</code>\n\
         3. Tags : {}\n\n\
         Quand c'est fait :\n<code>{}</code>",
        escape_html(&meta.title),
        escape_html(&meta.summary),
        escape_html(canonical_url),
        escape_html(&tag_list),
        escape_html(&mark_done),
    );
    let response = client
        .post(format!("https://api.telegram.org/bot{bot_token}/sendMessage"))
        .json(&json!({ "chat_id": require_env("TELEGRAM_CHAT_ID")?, "text": text, "parse_mode": "HTML" }))
        .send()
        .context("Telegram injoignable")?;
    check(response, "Telegram")?;
    Ok(PostRecord::now())
}

fn escape_html(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
