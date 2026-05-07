use comrak::adapters::SyntaxHighlighterAdapter;
use rust_embed::RustEmbed;
use std::{io::Cursor, io::Write as IoWrite, path::Path, sync::LazyLock};
use syntect::{
    highlighting::ThemeSet,
    html::{ClassStyle, css_for_theme_with_class_style, line_tokens_to_classed_spans},
    parsing::{ParseState, ScopeStack, SyntaxSet},
    util::LinesWithEndings,
};

const MAX_FILE_SIZE: usize = 512 * 1024;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct ThemeAssets;

fn load_all_themes() -> ThemeSet {
    let mut ts = ThemeSet::load_defaults();

    for file_path in ThemeAssets::iter() {
        if file_path.ends_with(".tmTheme") {
            if let Some(embedded_file) = ThemeAssets::get(&file_path) {
                let mut reader = Cursor::new(embedded_file.data);

                if let Ok(theme) = ThemeSet::load_from_reader(&mut reader) {
                    let theme_name = file_path
                        .strip_suffix(".tmTheme")
                        .unwrap_or(&file_path)
                        .to_string();

                    ts.themes.insert(theme_name, theme);
                }
            }
        }
    }

    ts
}

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(load_all_themes);

pub fn prime_syntax_set() {
    let _ = SYNTAX_SET.syntaxes().len();
}

pub fn light_highlight_css() -> &'static str {
    static CSS: LazyLock<Box<str>> = LazyLock::new(|| {
        let theme = &THEME_SET.themes["latte"];
        Box::from(css_for_theme_with_class_style(theme, ClassStyle::Spaced).unwrap())
    });
    &CSS
}

pub fn dark_highlight_css() -> &'static str {
    static CSS: LazyLock<Box<str>> = LazyLock::new(|| {
        let theme = &THEME_SET.themes["macchiato"];
        Box::from(css_for_theme_with_class_style(theme, ClassStyle::Spaced).unwrap())
    });
    &CSS
}

pub struct ComrakHighlightAdapter;

impl SyntaxHighlighterAdapter for ComrakHighlightAdapter {
    fn write_highlighted(
        &self,
        output: &mut dyn IoWrite,
        lang: Option<&str>,
        code: &str,
    ) -> std::io::Result<()> {
        let out = format_file(code, FileIdentifier::Token(lang.unwrap_or_default()))
            .map_err(std::io::Error::other)?;
        output.write_all(out.as_bytes())
    }

    fn write_pre_tag(
        &self,
        output: &mut dyn IoWrite,
        _attributes: std::collections::HashMap<String, String>,
    ) -> std::io::Result<()> {
        write!(output, "<pre>")
    }

    fn write_code_tag(
        &self,
        _output: &mut dyn IoWrite,
        _attributes: std::collections::HashMap<String, String>,
    ) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
pub enum FileIdentifier<'a> {
    Path(&'a Path),
    Token(&'a str),
}

pub fn format_file(content: &str, identifier: FileIdentifier<'_>) -> anyhow::Result<String> {
    let mut out = String::new();
    format_file_inner(&mut out, content, identifier, true)?;
    Ok(out)
}

pub fn format_file_inner(
    out: &mut String,
    content: &str,
    identifier: FileIdentifier<'_>,
    code_tag: bool,
) -> anyhow::Result<()> {
    let syntax = match identifier {
        FileIdentifier::Path(v) => find_syntax(v),
        FileIdentifier::Token(v) => find_syntax_by_token(v),
    };

    let line_prefix = if code_tag { "<code>" } else { "" };
    let line_suffix = if code_tag { "</code>" } else { "" };

    let Some(syntax) = syntax else {
        for line in content.lines() {
            out.push_str(line_prefix);
            v_htmlescape::b_escape(line.as_bytes(), out);
            out.push_str(line_suffix);
        }
        return Ok(());
    };

    if content.len() > MAX_FILE_SIZE {
        for line in content.lines() {
            out.push_str(line_prefix);
            v_htmlescape::b_escape(line.as_bytes(), out);
            out.push_str(line_suffix);
        }
        return Ok(());
    }

    let mut parse_state = ParseState::new(syntax);
    let mut scope_stack = ScopeStack::new();

    for line in LinesWithEndings::from(content) {
        out.push_str(line_prefix);
        match parse_state.parse_line(line, &SYNTAX_SET) {
            Ok(ops) => {
                match line_tokens_to_classed_spans(line, &ops, ClassStyle::Spaced, &mut scope_stack)
                {
                    Ok((html, _)) => out.push_str(&html),
                    Err(_) => v_htmlescape::b_escape(line.as_bytes(), out),
                }
            }
            Err(_) => v_htmlescape::b_escape(line.as_bytes(), out),
        }
        out.push_str(line_suffix);
    }

    Ok(())
}

fn find_syntax(file: &Path) -> Option<&'static syntect::parsing::SyntaxReference> {
    file.extension()
        .and_then(std::ffi::OsStr::to_str)
        .and_then(|ext| SYNTAX_SET.find_syntax_by_extension(ext))
}

fn find_syntax_by_token(token: &str) -> Option<&'static syntect::parsing::SyntaxReference> {
    if token.is_empty() {
        return None;
    }
    SYNTAX_SET
        .find_syntax_by_token(token)
        .or_else(|| SYNTAX_SET.find_syntax_by_name(token))
}
