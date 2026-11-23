## md2nexus - mym Edit Changelog

##### 0.5.0

- Added: The "--base-url" option, prefixing relative links with your repository's address, since a Nexus page has no files next to it
- Changed: Links to a section of the page written as plain text, Nexus pages having no anchors
- Fixed: A text looking like a Nexus tag, such as t[i] in inline code, read as markup by Nexus
- Fixed: Building from source with cargo install failing on a newer version of a dependency
- Improved: Release archives in zip for every platform, a format Nexus accepts

##### 0.4.0

- Added: Color themes, with the "--theme" option (gold, ember, frost, moss, arcane), coloring the headings and the header row of tables
- Changed: Renamed md2nexus - mym Edit, the command staying md2nexus
- Improved: The release archives include the changelog
- Improved: Release archives named after their platform (linux-x86_64, windows-x86_64, macos-x86_64, macos-arm64)

##### 0.3.0

First version of this edit.

- Added: Prebuilt executables for Linux, Windows and macOS (Intel and Apple Silicon) attached to each release
- Changed: Tables rendered with Nexus's table tags instead of aligned text in a code block, keeping the markup of cells and the alignment of columns, the header row in bold
- Fixed: Nested lists starting after an empty line on the Nexus page

##### 0.2.1 and Earlier

Versions by C J Silverio, see the original repository: ceejbot/md2nexus on GitHub.
