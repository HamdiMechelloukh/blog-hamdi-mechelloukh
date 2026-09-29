//! Markdown -> HTML, avec coloration syntaxique au build (classes CSS, aucun JS côté client).

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};
use syntect::highlighting::ThemeSet;
use syntect::html::{ClassStyle, ClassedHTMLGenerator, css_for_theme_with_class_style};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

const CLASS_STYLE: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "hl-" };
const THEME: &str = "base16-ocean.dark";

pub struct MarkdownRenderer {
    syntaxes: SyntaxSet,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        MarkdownRenderer { syntaxes: SyntaxSet::load_defaults_newlines() }
    }

    pub fn render(&self, markdown: &str) -> String {
        let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
        let mut events = Vec::new();
        // Langue du bloc de code en cours : Some tant qu'on est dans un bloc fenced.
        let mut code_lang: Option<String> = None;
        let mut code_text = String::new();

        for event in Parser::new_ext(markdown, options) {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    code_lang = Some(match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    });
                    code_text.clear();
                }
                Event::Text(text) if code_lang.is_some() => code_text.push_str(&text),
                Event::End(TagEnd::CodeBlock) => {
                    let lang = code_lang.take().unwrap_or_default();
                    events.push(Event::Html(self.highlight(&code_text, &lang).into()));
                }
                other => events.push(other),
            }
        }

        let mut output = String::new();
        html::push_html(&mut output, events.into_iter());
        output
    }

    fn highlight(&self, code: &str, lang: &str) -> String {
        let syntax = self
            .syntaxes
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text());
        let mut generator = ClassedHTMLGenerator::new_with_class_style(syntax, &self.syntaxes, CLASS_STYLE);
        for line in LinesWithEndings::from(code) {
            // Les grammaires par défaut de syntect ne produisent pas d'erreur sur du texte valide UTF-8.
            generator
                .parse_html_for_line_which_includes_newline(line)
                .expect("coloration syntaxique");
        }
        format!("<pre class=\"hl-code\"><code>{}</code></pre>\n", generator.finalize())
    }
}

/// Feuille de style des classes de coloration, écrite une fois dans dist/.
pub fn syntax_css() -> String {
    let themes = ThemeSet::load_defaults();
    css_for_theme_with_class_style(&themes.themes[THEME], CLASS_STYLE).expect("CSS du thème syntect")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fenced_code_is_highlighted_and_escaped() {
        let html = MarkdownRenderer::new().render("```python\nx = \"<b>\"\n```\n");
        assert!(html.contains("<pre class=\"hl-code\">"));
        assert!(html.contains("hl-"));
        assert!(html.contains("&lt;b&gt;"));
    }

    #[test]
    fn tables_are_rendered() {
        let html = MarkdownRenderer::new().render("| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(html.contains("<table>"));
    }
}
