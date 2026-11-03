#!/usr/bin/env python3
"""Download the rendered LSP 3.17 specification and convert it to Markdown.

The published page expands Jekyll includes from several directories; downloading
its Markdown source directly would leave most of the specification missing.
Requires pandoc.
"""

from common import PROJECT_DIR, extract_specification, fetch, to_markdown

EDITION = "3.17"
BASE_URL = f"https://microsoft.github.io/language-server-protocol/specifications/lsp/{EDITION}"
SOURCE = f"{BASE_URL}/specification/"


def specification_html(page):
    return extract_specification(page, ("id", "markdown-content-container"), "LSP")


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
