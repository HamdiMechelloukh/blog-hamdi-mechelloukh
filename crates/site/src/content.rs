//! Chargement et validation du contenu : articles markdown, données TOML, traductions.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::markdown::MarkdownRenderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Fr,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Fr, Lang::En];

    pub fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::En => "en",
        }
    }

    pub fn other(self) -> Lang {
        match self {
            Lang::Fr => Lang::En,
            Lang::En => Lang::Fr,
        }
    }

    /// Préfixe d'URL des pages de l'interface : le français est à la racine.
    pub fn prefix(self) -> &'static str {
        match self {
            Lang::Fr => "",
            Lang::En => "/en",
        }
    }

    pub fn og_locale(self) -> &'static str {
        match self {
            Lang::Fr => "fr_FR",
            Lang::En => "en_US",
        }
    }

    /// "2026-06-17" -> "17 juin 2026" / "17 June 2026" (équivalent de toLocaleDateString fr-FR / en-GB).
    pub fn long_date(self, iso_date: &str) -> String {
        const MONTHS_FR: [&str; 12] = [
            "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre",
            "octobre", "novembre", "décembre",
        ];
        const MONTHS_EN: [&str; 12] = [
            "January", "February", "March", "April", "May", "June", "July", "August", "September",
            "October", "November", "December",
        ];
        let mut parts = iso_date.splitn(3, '-');
        let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
            return iso_date.to_string();
        };
        let Some(month_name) = month.parse::<usize>().ok().and_then(|m| {
            let months = if self == Lang::Fr { MONTHS_FR } else { MONTHS_EN };
            months.get(m.checked_sub(1)?).copied()
        }) else {
            return iso_date.to_string();
        };
        let day = day.get(..2).unwrap_or(day).trim_start_matches('0');
        format!("{day} {month_name} {year}")
    }
}

/// Frontmatter TOML d'un article.
#[derive(Debug, Deserialize)]
pub struct ArticleMeta {
    pub translation_slug: String,
    pub lang: Lang,
    pub title: String,
    pub summary: String,
    pub date: String,
    pub tags: Vec<String>,
    pub reading_time_minutes: u32,
}

/// Article tel qu'écrit sur disque (partagé avec le crossposter).
#[derive(Debug)]
pub struct ArticleSource {
    pub slug: String,
    pub meta: ArticleMeta,
    /// Corps markdown, sans le frontmatter.
    pub markdown: String,
}

#[derive(Debug)]
pub struct Article {
    pub slug: String,
    pub translation_slug: String,
    pub lang: Lang,
    pub title: String,
    pub summary: String,
    pub date: String,
    pub tags: Vec<String>,
    pub reading_time_minutes: u32,
    pub html: String,
}

impl Article {
    pub fn path(&self) -> String {
        format!("/blog/{}", self.slug)
    }
}

#[derive(Debug, Deserialize)]
pub struct Project {
    title_fr: String,
    title_en: String,
    description_fr: String,
    description_en: String,
    pub technologies: Vec<String>,
    pub image_url: String,
    pub github_url: Option<String>,
    pub demo_url: Option<String>,
}

impl Project {
    pub fn title(&self, lang: &Lang) -> &str {
        match lang {
            Lang::Fr => &self.title_fr,
            Lang::En => &self.title_en,
        }
    }

    pub fn description(&self, lang: &Lang) -> &str {
        match lang {
            Lang::Fr => &self.description_fr,
            Lang::En => &self.description_en,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Experience {
    pub company: String,
    pub role: String,
    pub period: String,
    pub location: String,
    pub description: String,
    pub technologies: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Education {
    pub school: String,
    pub degree: String,
    pub period: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Feed {
    pub name: String,
    pub url: String,
}

#[derive(Deserialize)]
struct ProjectsFile {
    projects: Vec<Project>,
}

#[derive(Deserialize)]
struct AboutFile {
    experiences: Vec<Experience>,
    education: Vec<Education>,
}

#[derive(Deserialize)]
struct FeedsFile {
    feeds: Vec<Feed>,
}

/// Textes de l'interface. `deny_unknown_fields` : une clé orpheline dans le TOML fait échouer le build.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct I18n {
    pub nav: NavText,
    pub home: HomeText,
    pub about: AboutText,
    pub portfolio: PortfolioText,
    pub articles: ArticlesText,
    pub blog: WatchText,
    pub contact: ContactText,
    pub footer: FooterText,
    pub meta: MetaText,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavText {
    pub home: String,
    pub about: String,
    pub projects: String,
    pub articles: String,
    pub blog: String,
    pub contact: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HomeText {
    pub subtitle: String,
    pub description: String,
    pub cta_projects: String,
    pub cta_contact: String,
    pub skills_title: String,
    pub skill_data: String,
    pub skill_backend: String,
    pub skill_db: String,
    pub skill_cloud: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AboutText {
    pub title: String,
    pub who_title: String,
    pub who_text: String,
    pub journey_title: String,
    pub journey_text: String,
    pub interests_title: String,
    pub interest_karate_title: String,
    pub interest_karate_text: String,
    pub interest_gaming_title: String,
    pub interest_gaming_text: String,
    pub interest_pop_title: String,
    pub interest_pop_text: String,
    pub experience_title: String,
    pub education_title: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortfolioText {
    pub title: String,
    pub description: String,
    pub code: String,
    pub demo: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArticlesText {
    pub title: String,
    pub description: String,
    pub reading_time: String,
    pub back_to_list: String,
    pub published_on: String,
    pub empty: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchText {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactText {
    pub title: String,
    pub description: String,
    pub name: String,
    pub email: String,
    pub message: String,
    pub send: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FooterText {
    pub rights: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetaText {
    pub home_title: String,
    pub home_description: String,
    pub about_title: String,
    pub about_description: String,
    pub portfolio_title: String,
    pub portfolio_description: String,
    pub blog_title: String,
    pub watch_title: String,
    pub watch_description: String,
    pub contact_title: String,
    pub contact_description: String,
}

pub struct Content {
    /// Triés du plus récent au plus ancien.
    pub articles: Vec<Article>,
    pub projects: Vec<Project>,
    pub experiences: Vec<Experience>,
    pub education: Vec<Education>,
    pub feeds: Vec<Feed>,
    pub i18n_fr: I18n,
    pub i18n_en: I18n,
}

impl Content {
    pub fn load(content_dir: &Path, renderer: &MarkdownRenderer) -> Result<Self> {
        let mut articles = load_articles(&content_dir.join("articles"), renderer)?;
        articles.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.slug.cmp(&b.slug)));
        validate_translations(&articles)?;

        let data_dir = content_dir.join("data");
        let projects: ProjectsFile = read_toml(&data_dir.join("projects.toml"))?;
        let about: AboutFile = read_toml(&data_dir.join("about.toml"))?;
        let feeds: FeedsFile = read_toml(&data_dir.join("feeds.toml"))?;

        Ok(Content {
            articles,
            projects: projects.projects,
            experiences: about.experiences,
            education: about.education,
            feeds: feeds.feeds,
            i18n_fr: read_toml(&content_dir.join("i18n/fr.toml"))?,
            i18n_en: read_toml(&content_dir.join("i18n/en.toml"))?,
        })
    }

    pub fn i18n(&self, lang: Lang) -> &I18n {
        match lang {
            Lang::Fr => &self.i18n_fr,
            Lang::En => &self.i18n_en,
        }
    }

    pub fn article(&self, slug: &str) -> Option<&Article> {
        self.articles.iter().find(|a| a.slug == slug)
    }

    pub fn articles_in(&self, lang: Lang) -> impl Iterator<Item = &Article> {
        self.articles.iter().filter(move |a| a.lang == lang)
    }
}

fn read_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).with_context(|| format!("lecture de {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("TOML invalide : {}", path.display()))
}

fn load_articles(dir: &Path, renderer: &MarkdownRenderer) -> Result<Vec<Article>> {
    Ok(load_article_sources(dir)?
        .into_iter()
        .map(|source| Article {
            html: renderer.render(&source.markdown),
            slug: source.slug,
            translation_slug: source.meta.translation_slug,
            lang: source.meta.lang,
            title: source.meta.title,
            summary: source.meta.summary,
            date: source.meta.date,
            tags: source.meta.tags,
            reading_time_minutes: source.meta.reading_time_minutes,
        })
        .collect())
}

pub fn load_article_sources(dir: &Path) -> Result<Vec<ArticleSource>> {
    let mut sources = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("lecture de {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let slug = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .context("nom de fichier non UTF-8")?
            .to_string();
        let text = fs::read_to_string(&path)?;
        let (frontmatter, markdown) =
            split_frontmatter(&text).with_context(|| format!("frontmatter manquant : {}", path.display()))?;
        let meta: ArticleMeta =
            toml::from_str(frontmatter).with_context(|| format!("frontmatter invalide : {}", path.display()))?;
        sources.push(ArticleSource { slug, meta, markdown: markdown.to_string() });
    }
    Ok(sources)
}

/// Sépare le bloc `+++ ... +++` du corps markdown.
pub fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("+++\n")?;
    let end = rest.find("\n+++")?;
    let body = rest[end + 4..].trim_start_matches('\n');
    Some((&rest[..end], body))
}

/// Chaque article doit avoir un jumeau dans l'autre langue qui pointe vers lui en retour.
fn validate_translations(articles: &[Article]) -> Result<()> {
    let mut seen = HashSet::new();
    for article in articles {
        if !seen.insert(&article.slug) {
            bail!("slug en double : {}", article.slug);
        }
        let Some(twin) = articles.iter().find(|a| a.slug == article.translation_slug) else {
            bail!("{} : translation_slug {} introuvable", article.slug, article.translation_slug);
        };
        if twin.lang == article.lang || twin.translation_slug != article.slug {
            bail!("{} et {} ne sont pas des traductions réciproques", article.slug, twin.slug);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_date_matches_browser_locales() {
        assert_eq!(Lang::Fr.long_date("2026-06-07"), "7 juin 2026");
        assert_eq!(Lang::En.long_date("2026-03-19"), "19 March 2026");
    }

    #[test]
    fn frontmatter_is_split_from_body() {
        let (frontmatter, body) = split_frontmatter("+++\nlang = \"fr\"\n+++\n\n# Titre\n").unwrap();
        assert_eq!(frontmatter, "lang = \"fr\"");
        assert_eq!(body, "# Titre\n");
    }
}
