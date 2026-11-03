#!/usr/bin/env python3
"""Download the JSON-RPC 2.0 specification and convert it to Markdown.

Requires pandoc.
"""

import re

from common import PROJECT_DIR, extract_specification, fetch, to_markdown

EDITION = "2.0"
SOURCE = "https://www.jsonrpc.org/specification"


def specification_html(page):
    html = extract_specification(page, ("class", "span9"), "JSON-RPC")
    # The publisher uses bare pre blocks with br tags; pandoc otherwise loses the code formatting.
    html = re.sub(r"<pre>(.*?)</pre>", example_html, html, flags=re.DOTALL)
    return re.sub(r'(<h[1-6]\b[^>]*\bid="([^"]+)"[^>]*>)', r'<a id="\2"></a>\1', html)


def example_html(match):
    contents = re.sub(r"<br\s*/?>", "\n", match[1]).replace("&nbsp;", " ")
    return f"<pre><code>{contents}</code></pre>"


def convert(page):
    return to_markdown(specification_html(page))


def main():
    print(f"=== Downloading JSON-RPC Specification ({EDITION}) ===")
    markdown = convert(fetch(SOURCE))
    out_dir = PROJECT_DIR / "docs" / "lang-specs" / "jsonrpc" / EDITION
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "specification.md").write_text(markdown)
    (out_dir / "README.md").write_text(
        f"# JSON-RPC Specification ({EDITION})\n\n"
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
