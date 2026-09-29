//! Crossposter : publie les articles EN du blog vers dev.to et LinkedIn,
//! et envoie une notification Telegram pour importer manuellement sur Medium.
//!
//! Usage (depuis la racine du repo) :
//!   cargo run -p crossposter                                  -> poste le prochain item en queue
//!   cargo run -p crossposter -- --dry-run                     -> affiche ce qui serait posté
//!   cargo run -p crossposter -- --list                        -> affiche la queue
//!   cargo run -p crossposter -- --mark-done <platform> <slug> -> marque une plateforme comme traitée
//!   cargo run -p crossposter -- linkedin-auth                 -> obtient les tokens LinkedIn (une fois, en local)
//!
//! Environnement requis (selon plateforme) :
//!   DEVTO_API_KEY
//!   LINKEDIN_USER_URN + (LINKEDIN_REFRESH_TOKEN, LINKEDIN_CLIENT_ID, LINKEDIN_CLIENT_SECRET) ou LINKEDIN_ACCESS_TOKEN
//!   TELEGRAM_BOT_TOKEN, TELEGRAM_CHAT_ID

mod linkedin_auth;
mod platforms;
mod state;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use site::content::{ArticleSource, Lang, load_article_sources};

use state::{Platform, PostRecord, State};

const DEFAULT_BASE_URL: &str = "https://hamdimechelloukh.com";

pub struct QueueItem<'a> {
    pub article: &'a ArticleSource,
    pub platform: Platform,
}

/// File = couples (article, plateforme) pas encore traités, articles EN du plus ancien au plus récent.
fn build_queue<'a>(articles: &'a [ArticleSource], state: &State) -> Vec<QueueItem<'a>> {
    let mut english: Vec<&ArticleSource> = articles.iter().filter(|article| article.meta.lang == Lang::En).collect();
    english.sort_by(|a, b| a.meta.date.cmp(&b.meta.date));
    english
        .into_iter()
        .flat_map(|article| {
            Platform::ALL
                .into_iter()
                .filter(|platform| !state.is_done(&article.slug, *platform))
                .map(move |platform| QueueItem { article, platform })
        })
        .collect()
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // .env local facultatif ; en CI les secrets arrivent par l'environnement.
    let _ = dotenvy::from_path(root.join(".env"));
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.first().map(String::as_str) == Some("linkedin-auth") {
        return linkedin_auth::run();
    }

    let state_path = root.join(".crossposter-state.json");
    let mut state = State::load(&state_path)?;

    if let Some(index) = args.iter().position(|arg| arg == "--mark-done") {
        let (Some(platform), Some(slug)) =
            (args.get(index + 1).and_then(|name| Platform::parse(name)), args.get(index + 2))
        else {
            bail!("Usage : --mark-done <devto|linkedin|medium> <slug>");
        };
        state.record(slug, platform, PostRecord::now());
        state.save(&state_path)?;
        println!("✓ {slug}/{} marqué comme fait", platform.name());
        return Ok(());
    }

    let articles = load_article_sources(&root.join("content/articles"))?;
    let queue = build_queue(&articles, &state);

    if args.iter().any(|arg| arg == "--list") {
        println!("Queue : {} item(s)", queue.len());
        for item in &queue {
            println!("  - {} / {}", item.platform.name(), item.article.slug);
        }
        return Ok(());
    }
    let Some(next) = queue.first() else {
        println!("Queue vide — rien à publier.");
        return Ok(());
    };
    println!("→ Prochain : {} / {}", next.platform.name(), next.article.slug);

    let base_url = std::env::var("BLOG_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
    let canonical_url = format!("{base_url}/blog/{}", next.article.slug);
    if args.iter().any(|arg| arg == "--dry-run") {
        println!("[dry-run] pas d'envoi effectif.");
        if next.platform != Platform::Medium {
            println!("  URL : {canonical_url}");
        }
        return Ok(());
    }

    let result = platforms::publish(next, &canonical_url)?;
    let summary = result.url.clone().or_else(|| result.id.clone()).unwrap_or_else(|| "ok".to_string());
    state.record(&next.article.slug, next.platform, result);
    state.save(&state_path)?;
    println!("✓ {}/{} — {summary}", next.platform.name(), next.article.slug);
    Ok(())
}

pub fn require_env(name: &str) -> Result<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .with_context(|| format!("Variable d'environnement manquante : {name}"))
}
