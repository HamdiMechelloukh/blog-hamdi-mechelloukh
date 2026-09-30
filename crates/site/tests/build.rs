//! Build complet du vrai contenu dans un dossier temporaire, puis vérification du résultat.

use std::fs;
use std::path::{Path, PathBuf};

use site::{BASE_URL, BuildOptions, build};

fn build_site(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out_dir = std::env::temp_dir().join(format!("site-test-{name}-{}", std::process::id()));
    build(&BuildOptions { root, out_dir: out_dir.clone(), fetch_feeds: false, year: 2026 }).expect("build du site");
    out_dir
}

fn html_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(html_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "html") {
            files.push(path);
        }
    }
    files
}

/// Valeurs des attributs href="/..." et src="/..." (liens internes uniquement).
fn internal_links(html: &str) -> Vec<String> {
    let mut links = Vec::new();
    for attribute in ["href=\"/", "src=\"/"] {
        for (start, _) in html.match_indices(attribute) {
            let value_start = start + attribute.len() - 1;
            let value_end = value_start + html[value_start..].find('"').unwrap();
            links.push(html[value_start..value_end].to_string());
        }
    }
    links
}

/// Un chemin servi par Vercel : fichier exact, ou dossier contenant index.html.
fn resolves(out_dir: &Path, link: &str) -> bool {
    let path = out_dir.join(link.trim_start_matches('/'));
    path.is_file() || path.join("index.html").is_file()
}

#[test]
fn no_broken_internal_links() {
    let out_dir = build_site("links");
    let mut broken = Vec::new();
    for file in html_files(&out_dir) {
        for link in internal_links(&fs::read_to_string(&file).unwrap()) {
            if !resolves(&out_dir, &link) {
                broken.push(format!("{} -> {link}", file.strip_prefix(&out_dir).unwrap().display()));
            }
        }
    }
    fs::remove_dir_all(&out_dir).unwrap();
    assert!(broken.is_empty(), "liens cassés :\n{}", broken.join("\n"));
}

#[test]
fn sitemap_lists_every_page() {
    let out_dir = build_site("sitemap");
    let sitemap = fs::read_to_string(out_dir.join("sitemap.xml")).unwrap();
    // Pages servies par URL propre (dossier/index.html) ; 404.html est à part et hors sitemap.
    let pages: Vec<String> = html_files(&out_dir)
        .iter()
        .filter(|file| file.ends_with("index.html"))
        .map(|file| {
            let dir = file.parent().unwrap().strip_prefix(&out_dir).unwrap();
            format!("/{}", dir.display())
        })
        .collect();
    fs::remove_dir_all(&out_dir).unwrap();
    // 6 pages d'interface x 2 langues + 10 articles.
    assert_eq!(pages.len(), 22);
    for page in pages {
        let page = if page == "/" { page } else { page.trim_end_matches('/').to_string() };
        assert!(sitemap.contains(&format!("<loc>{BASE_URL}{page}</loc>")), "{page} absent du sitemap");
    }
}

#[test]
fn article_language_switch_points_to_its_twin() {
    let out_dir = build_site("switch");
    let html = fs::read_to_string(out_dir.join("blog/investment-bot-what-llms-taught-me/index.html")).unwrap();
    let not_found_fr = fs::read_to_string(out_dir.join("404.html")).unwrap();
    let not_found_en = fs::read_to_string(out_dir.join("en/404.html")).unwrap();
    fs::remove_dir_all(&out_dir).unwrap();
    for not_found in [&not_found_fr, &not_found_en] {
        assert!(not_found.contains("data-gpu-mode=\"game\"") && not_found.contains("noindex"));
    }
    assert!(not_found_fr.contains("<html lang=\"fr\">") && not_found_en.contains("<html lang=\"en\">"));
    // Mêmes effets que le reste du site : pas de mode particulier sur les articles.
    assert!(!html.contains("data-gpu-mode"));
    assert!(html.contains("<html lang=\"en\">"));
    assert!(html.contains("href=\"/blog/robot-investissement-ce-que-jai-appris-sur-les-llm\" class=\"lang-switch\""));
}
