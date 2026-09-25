#!/usr/bin/env python3
"""Download the rendered LSP 3.17 specification and convert it to Markdown.

The published page expands Jekyll includes from several directories; downloading
its Markdown source directly would leave most of the specification missing.
Requires pandoc.
"""

import re
from html.parser import HTMLParser

from common import PROJECT_DIR, fetch, to_markdown

EDITION = "3.17"
BASE_URL = f"https://microsoft.github.io/language-server-protocol/specifications/lsp/{EDITION}"
SOURCE = f"{BASE_URL}/specification/"


class SpecificationBody(HTMLParser):
    """Locate the rendered specification, excluding site navigation and footer."""

    def __init__(self):
        super().__init__()
        self.start = None
        self.end = None
        self.depth = 0

    def handle_starttag(self, tag, attrs):
        if self.start is None:
            if tag == "div" and ("id", "markdown-content-container") in attrs:
                self.start = self.getpos()
                self.depth = 1
        elif self.end is None and tag == "div":
            self.depth += 1

    def handle_endtag(self, tag):
        if tag == "div" and self.start is not None and self.end is None:
            self.depth -= 1
            if self.depth == 0:
                self.end = self.getpos()


def specification_html(page):
    parser = SpecificationBody()
    parser.feed(page)
    if parser.start is None or parser.end is None:
        raise ValueError("could not locate the rendered LSP specification")

    lines = page.splitlines(keepends=True)
    offsets = [0]
    for line in lines:
        offsets.append(offsets[-1] + len(line))

    def index(position):
        line, column = position
        return offsets[line - 1] + column

    start = page.index(">", index(parser.start)) + 1
    html = page[start:index(parser.end)]
    if not html.strip():
        raise ValueError("rendered LSP specification is empty")
    # GitHub Pages uses divs and spans for layout and syntax highlighting.
    # Preserve their contents (especially code), but not their presentation wrappers.
    return re.sub(r"</?(?:div|span)\b[^>]*>", "", html)


def convert(page):
    markdown = to_markdown(specification_html(page))
    # The meta model is a separate artifact, not downloaded with the spec.
    markdown = markdown.replace("(../metaModel/", f"({BASE_URL}/metaModel/")
    return markdown


def main():
    print(f"=== Downloading Language Server Protocol Specification ({EDITION}) ===")
    markdown = convert(fetch(SOURCE))
    out_dir = PROJECT_DIR / "docs" / "lang-specs" / "lsp" / EDITION
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "specification.md").write_text(markdown)
    (out_dir / "README.md").write_text(
        f"# Language Server Protocol Specification ({EDITION})\n\n"
        f"**Source:** {SOURCE}\n\n"
        "## Contents\n\n"
        "- [Specification](specification.md)\n"
    )
    print(
        f"    -> {out_dir.relative_to(PROJECT_DIR)}/specification.md "
        f"({markdown.count(chr(10))} lines)"
    )


if __name__ == "__main__":
    main()
