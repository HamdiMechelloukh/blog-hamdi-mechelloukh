//! Générateur statique du blog : content/ + static/ -> dist/.

pub mod content;
pub mod date;
pub mod feeds;
pub mod markdown;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use askama::Template;

use content::{Article, Content, Education, Experience, I18n, Lang, Project};
use feeds::WatchItem;
use markdown::MarkdownRenderer;

pub const BASE_URL: &str = "https://hamdimechelloukh.com";

/// Icônes de la navigation et des titres : SVG au trait insérés tels quels dans le HTML (`|safe`),
/// en `currentColor` pour suivre la couleur du lien (gris, orange quand la page est active).
pub mod icons {
    pub const HOME: &str = include_str!("../icons/home.svg");
    pub const ABOUT: &str = include_str!("../icons/about.svg");
    pub const PROJECTS: &str = include_str!("../icons/projects.svg");
    pub const BLOG: &str = include_str!("../icons/blog.svg");
    pub const WATCH: &str = include_str!("../icons/watch.svg");
    pub const CONTACT: &str = include_str!("../icons/contact.svg");
}
const FORMSPREE_URL: &str = "https://formspree.io/f/xlgwydgk";

pub struct BuildOptions {
    pub root: PathBuf,
    pub out_dir: PathBuf,
    /// false en test / hors ligne : la veille est alors vide.
    pub fetch_feeds: bool,
    /// Année du copyright du pied de page.
    pub year: i32,
}

pub struct Alternate {
    pub hreflang: &'static str,
    pub path: String,
}

pub struct NavItem {
    pub href: String,
    pub icon: &'static str,
    pub label: String,
    pub active: bool,
}

/// Tout ce que base.html a besoin de savoir sur la page rendue.
pub struct Layout<'a> {
    pub lang: Lang,
    pub t: &'a I18n,
    /// Clé de la section active dans la barre de navigation.
    pub section: &'static str,
    pub title: String,
    pub description: String,
    pub path: String,
    pub alternates: Vec<Alternate>,
    /// Même page dans l'autre langue.
    pub switch_path: String,
    pub og_type: &'static str,
    /// `<body data-gpu-mode>` : "" partout, "game" sur la page 404.
    pub gpu_mode: &'static str,
}

impl Layout<'_> {
    /// Chemin d'une page de l'interface dans la langue courante ("/about" -> "/en/about").
    pub fn href(&self, page: &str) -> String {
        localized(self.lang, page)
    }

    pub fn nav_items(&self) -> Vec<NavItem> {
        let nav = &self.t.nav;
        [
            ("home", "/", icons::HOME, &nav.home),
            ("about", "/about", icons::ABOUT, &nav.about),
            ("portfolio", "/portfolio", icons::PROJECTS, &nav.projects),
            ("blog", "/blog", icons::BLOG, &nav.articles),
            ("watch", "/veille", icons::WATCH, &nav.blog),
            ("contact", "/contact", icons::CONTACT, &nav.contact),
        ]
        .into_iter()
        .map(|(section, page, icon, label)| NavItem {
            href: self.href(page),
            icon,
            label: label.clone(),
            active: section == self.section,
        })
        .collect()
    }
}

fn localized(lang: Lang, page: &str) -> String {
    match (lang.prefix(), page) {
        ("", page) => page.to_string(),
        (prefix, "/") => prefix.to_string(),
        (prefix, page) => format!("{prefix}{page}"),
    }
}

/// hreflang de chaque langue + x-default (le français).
fn alternates(fr_path: String, en_path: String) -> Vec<Alternate> {
    vec![
        Alternate { hreflang: "fr", path: fr_path.clone() },
        Alternate { hreflang: "en", path: en_path },
        Alternate { hreflang: "x-default", path: fr_path },
    ]
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage<'a> {
    layout: Layout<'a>,
    year: i32,
}

#[derive(Template)]
#[template(path = "about.html")]
struct AboutPage<'a> {
    layout: Layout<'a>,
    year: i32,
    experiences: &'a [Experience],
    education: &'a [Education],
}

#[derive(Template)]
#[template(path = "portfolio.html")]
struct PortfolioPage<'a> {
    layout: Layout<'a>,
    year: i32,
    projects: &'a [Project],
}

#[derive(Template)]
#[template(path = "blog.html")]
struct BlogPage<'a> {
    layout: Layout<'a>,
    year: i32,
    articles: Vec<&'a Article>,
}

#[derive(Template)]
#[template(path = "article.html")]
struct ArticlePage<'a> {
    layout: Layout<'a>,
    year: i32,
    article: &'a Article,
}

#[derive(Template)]
#[template(path = "watch.html")]
struct WatchPage<'a> {
    layout: Layout<'a>,
    year: i32,
    items: &'a [WatchItem],
}

/// Page 404 par langue : /404.html et /en/404.html, choisies par préfixe dans vercel.json.
#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundPage<'a> {
    layout: Layout<'a>,
    year: i32,
}

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactPage<'a> {
    layout: Layout<'a>,
    year: i32,
}

pub struct SitemapEntry {
    pub path: String,
    pub alternates: Vec<Alternate>,
}

#[derive(Template)]
#[template(path = "sitemap.xml")]
struct Sitemap {
    entries: Vec<SitemapEntry>,
}

#[derive(Template)]
#[template(path = "rss.xml")]
struct RssFeed<'a> {
    lang: Lang,
    t: &'a I18n,
    blog_path: String,
    articles: Vec<&'a Article>,
}

/// Construit le site complet dans `options.out_dir` (vidé au préalable).
pub fn build(options: &BuildOptions) -> Result<()> {
    let renderer = MarkdownRenderer::new();
    let content = Content::load(&options.root.join("content"), &renderer)?;
    let watch_items = if options.fetch_feeds { feeds::fetch_all(&content.feeds) } else { Vec::new() };

    let out = &options.out_dir;
    if out.exists() {
        fs::remove_dir_all(out).with_context(|| format!("nettoyage de {}", out.display()))?;
    }
    copy_dir(&options.root.join("static"), out)?;
    fs::write(out.join("syntax.css"), markdown::syntax_css())?;

    let year = options.year;
    let mut sitemap = Vec::new();

    for lang in Lang::ALL {
        let t = content.i18n(lang);
        let meta = &t.meta;
        let ui_layout = |section, page: &str, title: &str, description: &str| Layout {
            lang,
            t,
            section,
            title: title.to_string(),
            description: description.to_string(),
            path: localized(lang, page),
            alternates: alternates(localized(Lang::Fr, page), localized(Lang::En, page)),
            switch_path: localized(lang.other(), page),
            og_type: "website",
            gpu_mode: "",
        };

        let pages: Vec<(&str, String)> = vec![
            ("/", HomePage { layout: ui_layout("home", "/", &meta.home_title, &meta.home_description), year }.render()?),
            (
                "/about",
                AboutPage {
                    layout: ui_layout("about", "/about", &meta.about_title, &meta.about_description),
                    year,
                    experiences: &content.experiences,
                    education: &content.education,
                }
                .render()?,
            ),
            (
                "/portfolio",
                PortfolioPage {
                    layout: ui_layout("portfolio", "/portfolio", &meta.portfolio_title, &meta.portfolio_description),
                    year,
                    projects: &content.projects,
                }
                .render()?,
            ),
            (
                "/blog",
                BlogPage {
                    layout: ui_layout("blog", "/blog", &meta.blog_title, &t.articles.description),
                    year,
                    articles: content.articles_in(lang).collect(),
                }
                .render()?,
            ),
            (
                "/veille",
                WatchPage {
                    layout: ui_layout("watch", "/veille", &meta.watch_title, &meta.watch_description),
                    year,
                    items: &watch_items,
                }
                .render()?,
            ),
            (
                "/contact",
                ContactPage {
                    layout: ui_layout("contact", "/contact", &meta.contact_title, &meta.contact_description),
                    year,
                }
                .render()?,
            ),
        ];
        for (page, html) in pages {
            write_page(out, &localized(lang, page), &html)?;
            sitemap.push(SitemapEntry {
                path: localized(lang, page),
                alternates: alternates(localized(Lang::Fr, page), localized(Lang::En, page)),
            });
        }

        let rss = RssFeed { lang, t, blog_path: localized(lang, "/blog"), articles: content.articles_in(lang).collect() };
        fs::write(out.join(localized(lang, "/rss.xml").trim_start_matches('/')), rss.render()?)?;
    }

    for article in &content.articles {
        // Le jumeau existe : Content::load a validé les paires de traductions.
        let twin = content.article(&article.translation_slug).expect("traduction validée");
        let (fr, en) = if article.lang == Lang::Fr { (article, twin) } else { (twin, article) };
        let layout = Layout {
            lang: article.lang,
            t: content.i18n(article.lang),
            section: "blog",
            title: format!("{} – Hamdi Mechelloukh", article.title),
            description: article.summary.clone(),
            path: article.path(),
            alternates: alternates(fr.path(), en.path()),
            switch_path: twin.path(),
            og_type: "article",
            gpu_mode: "",
        };
        write_page(out, &article.path(), &ArticlePage { layout, year, article }.render()?)?;
        sitemap.push(SitemapEntry { path: article.path(), alternates: alternates(fr.path(), en.path()) });
    }

    for lang in Lang::ALL {
        let t = content.i18n(lang);
        let not_found = NotFoundPage {
            layout: Layout {
                lang,
                t,
                section: "404",
                title: format!("404 – {}", t.not_found.title),
                description: t.not_found.text.clone(),
                path: localized(lang, "/404"),
                alternates: Vec::new(),
                switch_path: localized(lang.other(), "/"),
                og_type: "website",
                gpu_mode: "game",
            },
            year,
        };
        fs::write(out.join(localized(lang, "/404.html").trim_start_matches('/')), not_found.render()?)?;
    }

    fs::write(out.join("sitemap.xml"), Sitemap { entries: sitemap }.render()?)?;
    Ok(())
}

/// "/about" -> dist/about/index.html, servi en /about par Vercel.
fn write_page(out: &Path, path: &str, html: &str) -> Result<()> {
    let dir = out.join(path.trim_start_matches('/'));
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("index.html"), html).with_context(|| format!("écriture de {path}"))
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from).with_context(|| format!("lecture de {}", from.display()))? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
