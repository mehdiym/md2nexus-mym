use std::fs::{create_dir_all, File};
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use clap_complete::{generate, Shell};
use markdown::mdast::{AlignKind, Definition, FootnoteDefinition, Node, Table};
use markdown::{to_mdast, ParseOptions};
use owo_colors::OwoColorize;

#[derive(Clone, Debug, Parser)]
#[clap(name = "md2nexus", version = concat!(env!("CARGO_PKG_VERSION"), " (mym Edit)"))]
/// md2nexus - mym Edit: a command-line tool to convert GitHub-flavored Markdown to Nexus Mods BBCode.
pub struct Args {
    /// Path to an input file or directory. If omitted, input is read from stdin.
    #[clap(short, long, value_name = "PATHNAME")]
    input: Option<PathBuf>,
    /// Path to an output file or directory.
    ///
    /// If omitted, single-file input is written to stdout. If the input option is a directory,
    /// output files are written to '.'. If provided and input is a directory, output must also
    /// be a directory.
    #[clap(short, long, value_name = "PATHNAME")]
    output: Option<PathBuf>,
    /// Color theme: colors the headings and the header row of tables. All the colors are made to
    /// be read on the dark background of the Nexus.
    #[clap(short, long, value_enum, default_value_t = Theme::None)]
    theme: Theme,
    /// Base URL of relative links, such as a repository's
    /// 'https://github.com/<user>/<repo>/blob/<branch>/'.
    ///
    /// A Nexus page has no files next to it: without this option, a relative link is kept as is,
    /// with a warning, and leads nowhere there. Links to a section of the page ('#usage') become
    /// plain text, Nexus pages having no anchors.
    #[clap(short, long, value_name = "URL")]
    base_url: Option<String>,
    /// Emit completion data for your preferred shell.
    #[clap(short, long)]
    completions: Option<Shell>,
    /// Print GPL-3.0 license information.
    #[clap(long)]
    license: bool,
}

/// A color theme, made of two colors for a dark background.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Theme {
    /// No colors.
    #[default]
    None,
    /// Morrowind's interface: gold and parchment.
    Gold,
    /// Orange and amber.
    Ember,
    /// Ice blue.
    Frost,
    /// Moss green.
    Moss,
    /// Violet and lavender.
    Arcane,
}

impl Theme {
    /// The color of the main headings (`#` and `##`), then the softer one of the smaller headings
    /// and of the header row of tables. None for the theme without colors.
    fn colors(self) -> Option<(&'static str, &'static str)> {
        match self {
            Theme::None => None,
            // Morrowind's FontColor_color_normal and FontColor_color_header.
            Theme::Gold => Some(("#caa560", "#dfc99f")),
            Theme::Ember => Some(("#e8834e", "#f0b27a")),
            Theme::Frost => Some(("#6fb3e8", "#a9cfee")),
            Theme::Moss => Some(("#8fbf6a", "#b8d69e")),
            Theme::Arcane => Some(("#b18fe0", "#cbb4ec")),
        }
    }
}

/// The tags Nexus renders. BBCode has no escape: a text such as `t[i]` or `[table]` in inline
/// code would be read as a tag.
const NEXUS_TAGS: &[&str] = &[
    "*", "b", "center", "code", "color", "email", "font", "heading", "hr", "i", "img", "left",
    "line", "list", "quote", "right", "s", "size", "spoiler", "sub", "sup", "table", "td", "th",
    "tr", "u", "url", "youtube",
];

/// Break every Nexus tag found in a text with a zero-width space after its opening bracket, so
/// Nexus shows it as written instead of reading it as markup.
fn escape_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for (index, c) in text.char_indices() {
        result.push(c);
        if c != '[' {
            continue;
        }
        let rest = &text[index + 1..];
        let rest = rest.strip_prefix('/').unwrap_or(rest);
        let end = rest.find([']', '=']).unwrap_or(rest.len());
        if end < rest.len() && NEXUS_TAGS.contains(&rest[..end].to_lowercase().as_str()) {
            result.push('\u{200B}');
        }
    }
    result
}

/// Whether a URL is absolute: it has a scheme (`https:`, `mailto:`) or starts with `//`.
fn is_absolute(url: &str) -> bool {
    if url.starts_with("//") {
        return true;
    }
    match url.find(':') {
        Some(colon) => {
            let scheme = &url[..colon];
            scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        }
        None => false,
    }
}

/// Wrap a text in a color tag, or leave it as is without a color.
fn colored(text: String, color: Option<&str>) -> String {
    match color {
        Some(color) => format!("[color={color}]{text}[/color]"),
        None => text,
    }
}

/// Handle all the markdown files in a directory, converting each an.
/// This does not yet visit subdirs, because I haven't needed that use case.
/// This function assumes you've created the output dir already.
fn handle_directory(dirpath: PathBuf, outpath: PathBuf, options: &Options) -> anyhow::Result<()> {
    println!(
        "Converting all files in directory {}",
        dirpath.display().bold().blue()
    );
    for entry in std::fs::read_dir(dirpath)? {
        let entry = entry?;
        let fpath = entry.path();
        if !fpath.is_file() {
            continue;
        }
        let Some(filename) = fpath.file_name() else {
            continue;
        };
        let Some(ext) = fpath.extension() else {
            continue;
        };
        if ext != "md" {
            continue;
        }
        let outfname = outpath
            .clone()
            .join(filename.to_string_lossy().replace(".md", ".bbcode"));

        let mut data = String::new();
        let file = File::open(&fpath)?;
        let mut reader = BufReader::new(file);
        reader.read_to_string(&mut data)?;
        let result = convert_buffer(&data, options);
        let mut output = File::create(&outfname)?;
        write!(output, "{result}")?;
        println!(
            "    {} => {}",
            fpath.display().yellow(),
            outfname.display().bright_magenta()
        );
    }

    Ok(())
}

/// Given an input markdown str, emit nexus bbcode as a string.
fn convert_buffer(input: &str, options: &Options) -> String {
    // This function is infallible with the default options.
    let tree =
        to_mdast(input, &ParseOptions::gfm()).expect("failed to parse markdown as valid GFM.");
    let mut state = State::new(options.clone());
    state.render(tree)
}

/// What the command line sets for the conversion.
#[derive(Clone, Debug, Default)]
struct Options {
    theme: Theme,
    /// Prefixed to relative links.
    base_url: Option<String>,
}

/// State is the worst.
#[derive(Clone, Debug)]
struct State {
    /// Whether the list being rendered is tight (no blank line between its items).
    tight: bool,
    definitions: Vec<Definition>,
    footnotes: Vec<FootnoteDefinition>,
    theme: Theme,
    base_url: Option<String>,
}

impl State {
    pub fn new(Options { theme, base_url }: Options) -> Self {
        State {
            tight: false,
            definitions: Vec::new(),
            footnotes: Vec::new(),
            theme,
            base_url,
        }
    }

    pub fn render(&mut self, tree: Node) -> String {
        if let Some(children) = tree.children() {
            let rendered = self.render_nodes(children);
            let linkdefs = self
                .definitions
                .clone()
                .iter()
                .map(|def| {
                    let anchor = if let Some(ref title) = def.title {
                        title.clone()
                    } else {
                        def.identifier.clone()
                    };
                    format!(
                        "\n^{}: [url={}]{}[/url]",
                        def.identifier,
                        self.resolve(&def.url),
                        anchor
                    )
                })
                .collect::<Vec<String>>()
                .join("\n");
            let footnotes = self
                .footnotes
                .clone()
                .iter()
                .map(|xs| format!("\n^{}: {}", xs.identifier, self.render_nodes(&xs.children)))
                .collect::<Vec<String>>()
                .join("\n");
            let result = [rendered, linkdefs, footnotes].join("");
            // Sadly, some whitespace fixup because Nexus rendering is quite sensitive to it
            // and I didn't track enough state to get it right.
            result
                .trim()
                .replace("\n\n\n", "\n\n")
                .replace("[*]\n", "[*]")
        } else {
            "".to_string()
        }
    }

    /// Render the passed-in vector of nodes.
    pub fn render_nodes(&mut self, nodelist: &[Node]) -> String {
        nodelist
            .iter()
            .map(|xs| self.render_node(xs))
            .collect::<Vec<String>>()
            .join("")
    }

    /// The URL of a link or an image on the Nexus page: a relative one is prefixed with the base
    /// URL, or kept as is with a warning without one.
    fn resolve(&self, url: &str) -> String {
        if is_absolute(url) {
            return url.to_string();
        }
        let path = url
            .strip_prefix("./")
            .unwrap_or(url)
            .trim_start_matches('/');
        match self.base_url {
            Some(ref base) => format!("{}/{path}", base.trim_end_matches('/')),
            None => {
                eprintln!("warning: relative link '{url}' leads nowhere on Nexus, see --base-url");
                url.to_string()
            }
        }
    }

    /// Render an item of a tight list. Nexus turns every newline into a line break, so the
    /// item's text goes without the newlines of a paragraph: with them, a nested list would
    /// start after an empty line. A nested list follows the text directly, and nothing follows
    /// a nested list, which would leave an empty line after it.
    fn render_tight_item(&mut self, children: &[Node]) -> String {
        let mut result = String::from("[*]");
        for child in children {
            match child {
                Node::Paragraph(p) => result.push_str(&self.render_nodes(&p.children)),
                Node::List(_) => result.push_str(self.render_node(child).trim_start()),
                _ => result.push_str(&self.render_node(child)),
            }
        }
        if !result.ends_with("[/list]") {
            result.push('\n');
        }
        result
    }

    /// Render a table with Nexus's table tags. It goes on a single line: Nexus turns every newline
    /// into a line break, which has no place between rows or cells. A column's alignment applies
    /// to each of its cells. The header row is made of bold `[td]` cells rather than `[th]`,
    /// which Nexus renders without padding nor borders. A theme colors the header row like the
    /// smaller headings.
    fn render_table(&mut self, table: &Table) -> String {
        let mut result = String::from("\n[table]");
        for (index, row) in table.children.iter().enumerate() {
            let Node::TableRow(row) = row else {
                continue;
            };
            result.push_str("[tr]");
            for (column, cell) in row.children.iter().enumerate() {
                let content = self.render_node(cell);
                let content = if index == 0 {
                    let color = self.theme.colors().map(|(_, minor)| minor);
                    colored(format!("[b]{content}[/b]"), color)
                } else {
                    content
                };
                let content = match table.align.get(column) {
                    Some(AlignKind::Center) => format!("[center]{content}[/center]"),
                    Some(AlignKind::Right) => format!("[right]{content}[/right]"),
                    _ => content,
                };
                result.push_str(&format!("[td]{content}[/td]"));
            }
            result.push_str("[/tr]");
        }
        result.push_str("[/table]\n");
        result
    }

    /// Convert a single node type to bbcode, recursing into children if it has any.
    /// This is where all the interesting work is.
    pub fn render_node(&mut self, node: &Node) -> String {
        match node {
            Node::Root(root) => self.render_nodes(&root.children),
            Node::Paragraph(p) => format!("\n{}\n", self.render_nodes(&p.children)),
            Node::Text(t) => escape_tags(&t.value),
            Node::BlockQuote(t) => {
                format!("[quote]{}[/quote]\n", self.render_nodes(&t.children))
            }
            Node::List(list) => {
                let outer = self.tight;
                self.tight = !list.spread;
                let items = self.render_nodes(&list.children);
                self.tight = outer;
                let open = if list.ordered { "[list=1]" } else { "[list]" };
                format!("\n{open}\n{items}[/list]")
            }
            Node::ListItem(item) => {
                if self.tight && !item.spread {
                    self.render_tight_item(&item.children)
                } else {
                    format!("[*]{}", self.render_nodes(&item.children))
                }
            }
            Node::Break(_) => "\n\n".to_string(),
            Node::Code(t) => {
                format!("\n[code]{}[/code]\n", t.value)
            }
            Node::InlineCode(t) => format!("[font=Courier New]{}[/font]", escape_tags(&t.value)),
            Node::Delete(t) => format!("[s]{}[/s]", self.render_nodes(&t.children)),
            Node::Emphasis(t) => format!("[i]{}[/i]", self.render_nodes(&t.children)),
            Node::Strong(t) => format!("[b]{}[/b]", self.render_nodes(&t.children)),
            Node::Heading(h) => {
                // Nexus bbcode does not support "heading". SMH.
                let size = (7 - h.depth).clamp(2, 6);
                let color = self
                    .theme
                    .colors()
                    .map(|(major, minor)| if h.depth <= 2 { major } else { minor });
                let text = colored(self.render_nodes(&h.children), color);
                format!("\n[size={size}]{text}[/size]\n\n")
            }
            Node::Html(t) => t.value.clone(),
            Node::Image(t) => format!("[img]{}[/img]", self.resolve(&t.url)),
            Node::Link(link) => {
                let text = self.render_nodes(&link.children);
                if link.url.starts_with('#') {
                    text
                } else {
                    format!("[url={}]{text}[/url]", self.resolve(&link.url))
                }
            }
            Node::Math(t) => format!("\n[code]{}[/code]\n", t.value), // no equivalent in bbcode
            Node::InlineMath(t) => format!("[font=Courier New]{}[/font]", escape_tags(&t.value)),
            Node::ThematicBreak(_) => "\n\n[line]\n\n".to_string(),
            Node::Toml(t) => format!("\n[code]{}[/code]\n\n", t.value),
            Node::Yaml(t) => format!("\n[code]{}[/code]\n\n", t.value),

            // Tables
            Node::Table(table) => self.render_table(table),
            // Reached only through render_table, which renders rows and cells itself.
            Node::TableRow(row) => self.render_nodes(&row.children),
            Node::TableCell(cell) => self.render_nodes(&cell.children),

            // the following markup types have meh support
            Node::FootnoteReference(footie) => {
                format!("(See ^{})", footie.identifier)
            }
            Node::FootnoteDefinition(ref note) => {
                self.footnotes.push(note.clone());
                "".to_string()
            }
            Node::Definition(ref def) => {
                self.definitions.push(def.clone());
                "".to_string()
            }
            Node::ImageReference(imgref) => {
                format!("[{}] {}", imgref.identifier, imgref.alt)
            }
            Node::LinkReference(linkref) => {
                format!(
                    "(See image {}; {})",
                    linkref.identifier,
                    self.render_nodes(&linkref.children)
                )
            }

            // completely unsupported markup types follow
            Node::MdxTextExpression(_) => {
                eprintln!("mdx not supported in nexus bbcode");
                "".to_string()
            }
            Node::MdxFlowExpression(_) => {
                eprintln!("mdx not supported in nexus bbcode");
                "".to_string()
            }
            Node::MdxJsxFlowElement(_) => {
                eprintln!("mdx/jxs not supported in nexus bbcode");
                "".to_string()
            }
            Node::MdxjsEsm(_) => {
                eprintln!("mdx/jxs not supported in nexus bbcode");
                "".to_string()
            }
            Node::MdxJsxTextElement(_) => {
                eprintln!("mdx/jxs not supported in nexus bbcode");
                "".to_string()
            }
        }
    }
}

fn license() {
    println!(
        r#"
md2nexus Copyright (C) 2023 C J Silverio
mym Edit changes Copyright (C) 2026 Mehdi Yousfi-Monod

This program is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License, or (at your option) any later
version.

This program is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with
this program. If not, see <https://www.gnu.org/licenses/>.
"#
    );
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if let Some(shell) = args.completions {
        use clap::CommandFactory;
        let mut app = Args::command();
        generate(shell, &mut app, "md2nexus", &mut std::io::stdout());
        return Ok(());
    }

    if args.license {
        license();
        return Ok(());
    }

    let options = Options {
        theme: args.theme,
        base_url: args.base_url.clone(),
    };
    if let Some(ref input) = args.input {
        if input.is_dir() {
            let output = if let Some(ref out) = args.output {
                out.clone()
            } else {
                PathBuf::from(".")
            };
            if !output.exists() {
                create_dir_all(&output)?;
            }
            handle_directory(input.clone(), output, &options)?;
            return Ok(());
        }
    };

    // Not dealing with directories, only a single input file, which might be stdin.
    let mut data = String::new();
    if let Some(input) = args.input {
        let file = File::open(input.clone())?;
        let mut reader = BufReader::new(file);
        reader.read_to_string(&mut data)?;
    } else {
        let mut reader = BufReader::new(std::io::stdin());
        reader.read_to_string(&mut data)?;
    }
    let result = convert_buffer(&data, &options);
    if let Some(outpath) = args.output {
        let mut output = File::create(outpath)?;
        write!(output, "{result}")?;
    } else {
        println!("{result}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{convert_buffer, Options, Theme};

    fn convert(input: &str) -> String {
        convert_buffer(input, &Options::default())
    }

    #[test]
    fn nested_list_follows_its_item_text() {
        assert_eq!(
            convert("- a\n  - b\n- c\n"),
            "[list]\n[*]a[list]\n[*]b\n[/list][*]c\n[/list]"
        );
    }

    #[test]
    fn loose_list_keeps_its_paragraphs() {
        assert_eq!(convert("- a\n\n- b\n"), "[list]\n[*]a\n[*]b\n[/list]");
    }

    #[test]
    fn table_uses_table_tags() {
        assert_eq!(
            convert("| A | B |\n|:---|---:|\n| `x` | **y** |\n"),
            "[table][tr][td][b]A[/b][/td][td][right][b]B[/b][/right][/td][/tr]\
             [tr][td][font=Courier New]x[/font][/td][td][right][b]y[/b][/right][/td][/tr][/table]"
        );
    }

    #[test]
    fn theme_colors_headings_and_table_header() {
        assert_eq!(
            convert_buffer(
                "# A\n\n### B\n\n| C |\n|---|\n| d |\n",
                &Options {
                    theme: Theme::Gold,
                    ..Options::default()
                }
            ),
            "[size=6][color=#caa560]A[/color][/size]\n\n\
             [size=4][color=#dfc99f]B[/color][/size]\n\n\
             [table][tr][td][color=#dfc99f][b]C[/b][/color][/td][/tr][tr][td]d[/td][/tr][/table]"
        );
    }
    #[test]
    fn relative_links_get_the_base_url() {
        let options = Options {
            base_url: Some("https://example.com/repo/".to_string()),
            ..Options::default()
        };
        assert_eq!(
            convert_buffer(
                "[a](CHANGELOG.md) [b](./docs/x.md) [c](https://x.org) [d](#usage) [e](mailto:a@b.c)",
                &options
            ),
            "[url=https://example.com/repo/CHANGELOG.md]a[/url] \
             [url=https://example.com/repo/docs/x.md]b[/url] \
             [url=https://x.org]c[/url] d [url=mailto:a@b.c]e[/url]"
        );
    }

    #[test]
    fn tags_in_text_are_broken() {
        assert_eq!(
            convert("`t[i]` `[/table]` [x] `a[1]` `[color=red]`"),
            "[font=Courier New]t[\u{200B}i][/font] [font=Courier New][\u{200B}/table][/font] [x] \
             [font=Courier New]a[1][/font] [font=Courier New][\u{200B}color=red][/font]"
        );
    }
}
