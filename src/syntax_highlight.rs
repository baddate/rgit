use std::{
    io::{ErrorKind, Write as IoWrite},
    path::Path,
    sync::LazyLock,
};

use comrak::adapters::SyntaxHighlighterAdapter;
use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator, css_for_theme_with_class_style},
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

const MAX_FILE_SIZE: usize = 512 * 1024;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

pub fn prime_syntax_set() {
    let _ = SYNTAX_SET.len();
}

pub fn light_highlight_css() -> &'static str {
    static CSS: LazyLock<Box<str>> = LazyLock::new(|| {
        let theme = &THEME_SET.themes["InspiredGitHub"];
        Box::from(css_for_theme_with_class_style(theme, ClassStyle::Spaced).unwrap())
    });
    &CSS
}

pub fn dark_highlight_css() -> &'static str {
    static CSS: LazyLock<Box<str>> = LazyLock::new(|| {
        let theme = &THEME_SET.themes["base16-ocean.dark"];
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
            .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;
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
    let line_suffix = if code_tag { "</code>\n" } else { "\n" };

    if syntax.is_none() || content.len() > MAX_FILE_SIZE {
        for line in content.lines() {
            out.push_str(line_prefix);
            v_htmlescape::b_escape(line.as_bytes(), out);
            out.push_str(line_suffix);
        }
        return Ok(());
    }

    let syntax = syntax.unwrap();
    let mut html_generator =
        ClassedHTMLGenerator::new_with_class_style(syntax, &SYNTAX_SET, ClassStyle::Spaced);

    for line in LinesWithEndings::from(content) {
        out.push_str(line_prefix);
        match html_generator.parse_line_for_classed_html(line) {
            Ok(highlighted) => out.push_str(&highlighted),
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
    SYNTAX_SET.find_syntax_by_name(token)
}
