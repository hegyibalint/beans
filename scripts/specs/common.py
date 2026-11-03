"""Shared pipeline for fetching a spec and converting it to Markdown.

The JLS and JVMS entry scripts pin their editions and chapter tables;
this module owns their download -> pandoc (html -> gfm) -> cleanup pipeline.
Single-page specifications reuse fetching, body extraction, and conversion.
"""

import re
import subprocess
import time
import urllib.request
from html.parser import HTMLParser
from pathlib import Path

PROJECT_DIR = Path(__file__).resolve().parents[2]


def download_spec(prefix, edition, base_url, chapters, readme_header):
    out_dir = PROJECT_DIR / "docs" / "lang-specs" / prefix / edition
    out_dir.mkdir(parents=True, exist_ok=True)

    links = chapter_links(prefix, chapters)
    for num, slug, title in chapters:
        source = f"{base_url}/{prefix}-{num}.html"
        target = out_dir / chapter_file(num, slug)
        print(f"  Downloading {prefix}-{num}: {title}...")
        markdown = clean(to_markdown(fetch(source)), prefix, links)
        target.write_text(markdown)
        print(f"    -> {target.name} ({markdown.count(chr(10))} lines)")
        time.sleep(0.5)

    readme = out_dir / "README.md"
    readme.write_text(readme_header + "\n## Chapters\n\n" + toc(chapters))
    print(f"    -> {readme.relative_to(PROJECT_DIR)}")


def chapter_file(num, slug):
    return f"ch{num:02d}-{slug}.md"


def chapter_links(prefix, chapters):
    """Cross-chapter link rewrites: jls-6.html -> ch06-names.md"""
    return {
        f"{prefix}-{num}.html": chapter_file(num, slug)
        for num, slug, _title in chapters
    }


def toc(chapters):
    return "".join(
        f"{num}. [{title}]({chapter_file(num, slug)})\n"
        for num, slug, title in chapters
    )


def fetch(url):
    request = urllib.request.Request(url, headers={"User-Agent": "curl/8"})
    with urllib.request.urlopen(request) as response:
        return response.read().decode("utf-8")


class SpecificationBody(HTMLParser):
    """Locate a specification's div, excluding site navigation and footer."""

    def __init__(self, attribute):
        super().__init__()
        self.attribute = attribute
        self.start = None
        self.end = None
        self.depth = 0

    def handle_starttag(self, tag, attrs):
        if self.start is None:
            if tag == "div" and self.attribute in attrs:
                self.start = self.getpos()
                self.depth = 1
        elif self.end is None and tag == "div":
            self.depth += 1

    def handle_endtag(self, tag):
        if tag == "div" and self.start is not None and self.end is None:
            self.depth -= 1
            if self.depth == 0:
                self.end = self.getpos()


def extract_specification(page, attribute, name):
    parser = SpecificationBody(attribute)
    parser.feed(page)
    if parser.start is None or parser.end is None:
        raise ValueError(f"could not locate the rendered {name} specification")

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
        raise ValueError(f"rendered {name} specification is empty")
    # Remove layout and syntax-highlighting wrappers without losing their contents.
    return re.sub(r"</?(?:div|span)\b[^>]*>", "", html)


def to_markdown(html):
    return subprocess.run(
        ["pandoc", "-f", "html", "-t", "gfm", "--wrap=none"],
        input=html,
        capture_output=True,
        text=True,
        check=True,
    ).stdout


def clean(markdown, prefix, links):
    lines = [clean_line(line, prefix, links) for line in body(markdown)]
    return "\n".join(collapse_blanks(lines)) + "\n"


def body(markdown):
    """The chapter itself: from its '# Chapter' heading to Oracle's nav footer."""
    lines = []
    in_body = False
    for line in markdown.splitlines():
        if line.startswith('<div class="navfooter">'):
            break
        if line.startswith("# Chapter"):
            in_body = True
        if in_body:
            lines.append(line)
    return lines


def clean_line(line, prefix, links):
    line = re.sub(r"^\s*<div[^>]*>", "", line)
    line = line.replace("</div>", "")
    line = re.sub(r"<span[^>]*>", "", line)
    line = line.replace("</span>", "")
    line = line.replace("<em>", "").replace("</em>", "")
    line = re.sub(r"\s*®", "", line)
    line = line.replace("**Table of Contents**", "## Contents")
    line = re.sub(r' accesskey="[^"]*"', "", line)
    line = re.sub(r'<a href="([^"]*)"[^>]*>(.*?)</a>', r"[\2](\1)", line)
    for html_name, markdown_name in links.items():
        line = line.replace(html_name, markdown_name)
    line = re.sub(rf"{prefix}-[0-9]+\.html#", "#", line)
    return line


def collapse_blanks(lines):
    """Runs of three or more blank lines shrink to two."""
    kept = []
    blanks = 0
    for line in lines:
        blanks = blanks + 1 if line == "" else 0
        if blanks <= 2:
            kept.append(line)
    return kept
