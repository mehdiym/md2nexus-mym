# md2nexus

This is a command-line tool for converting single files or directories of files from Markdown to valid [Nexusmods](https://nexusmods.com) [bbcode](https://wiki.nexusmods.com/index.php/Formating_and_BBCode_in_Descriptions). The Nexus bbcode editors are hell to use if you're writing any complex documentation.

## About this fork

This is a fork of [ceejbot/md2nexus](https://github.com/ceejbot/md2nexus) by C J Silverio, maintained by Mehdi Yousfi-Monod since upstream went quiet after version 0.2.1. Changes since 0.2.1:

- Nested lists no longer start after an empty line on the Nexus page: in a tight list, an item's text and its nested list are no longer separated by newlines, each of which Nexus renders as a line break. Loose lists (blank lines between items) keep the previous rendering.
- Tables are rendered with Nexus's `[table]` tags instead of aligned text in a code block: markup inside cells (bold, links, inline code) is kept, each column's alignment is kept through `[center]` and `[right]`, and the header row is bold. The `prettytable` dependency is gone.

## Installation

Prebuilt executables for Linux, Windows and macOS (Intel and Apple Silicon) are attached to [the releases](https://github.com/mehdiym/md2nexus/releases/latest) of this fork; those of upstream and its Homebrew tap don't include the changes above. With Rust installed, you can also build from source:

```shell
git clone https://github.com/mehdiym/md2nexus
cd md2nexus
cargo install --path .

# once installed:
md2nexus --help
```

## Usage

Simplest possible usage:

```shell
cat docs.md | md2nexus > docs.bbcode
# or
cat docs.md | md2nexus | pbcopy
# and then paste into the nexus editor
```

If you don't specify an input file, `md2nexus` reads stdin. If you don't specify an output file, `md2nexus` writes the converted text to stdout. It prints nothing to stdout other than program output.

If the input option is a _directory_, `md2nexus` converts all markdown files in the input directory and writes them to the output option, which must be a directory. If you do not specify an output directory, it writes them to the current directory. It prints the names of converted files to stdout.

## Notes on conversion

Markdown is far more expressive than Nexus bbcode, which is a stunted bbcode variation. This tool converts some Markdown concepts to code blocks to make them work. Markdown tables become Nexus `[table]` tags, the header row in bold, with each column's alignment kept through `[center]` and `[right]`.

Github-flavored markdown is supported. [MDX](https://mdxjs.com) is not supported at all, as Nexus bbcode doesn't allow components or javascript or indeed any potentially dangerous user-generated content.

In-document references such as footnotes or link references are not converted into usable links, but they are emitted into the text. [^note1]

## Full usage

```text
$ md2nexus -h
A command-line tool to convert gfm markdown to NexusMods-flavored bbcode

Usage: md2nexus [OPTIONS]

Options:
  -i, --input <PATHNAME>           Path to an input file or directory.
                                   If omitted, input is read from stdin.
  -o, --output <PATHNAME>          Path to an output file or directory
  -c, --completions <COMPLETIONS>  Emit completion data for your preferred shell [possible values:
                                   bash, elvish, fish, powershell, zsh]
      --license                    Print GPL-3.0 license information
  -h, --help                       Print help (see more with '--help')
  -V, --version                    Print version
```

## Implementation notes

The heavy lifting is done by the [markdown](https://lib.rs/crates/markdown) crate, which generates an AST for input documents. The tool visits the tree's nodes and generates appropriate bbcode.

[^note1]: This is an example footnote. It's better than nothing?

## LICENSE

GPL-3.0. Copyright (C) 2023 C J Silverio; fork changes copyright (C) 2026 Mehdi Yousfi-Monod.
