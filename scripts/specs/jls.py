#!/usr/bin/env python3
"""Download the Java Language Specification and convert it to Markdown.

The edition is pinned; bump it deliberately, the spec-test citations
follow it. Requires pandoc.
"""

from common import download_spec

EDITION = "se26"
BASE_URL = f"https://docs.oracle.com/javase/specs/jls/{EDITION}/html"

CHAPTERS = [
    (1, "introduction", "Introduction"),
    (2, "grammars", "Grammars"),
    (3, "lexical-structure", "Lexical Structure"),
    (4, "types-values-variables", "Types, Values, and Variables"),
    (5, "conversions-contexts", "Conversions and Contexts"),
    (6, "names", "Names"),
    (7, "packages-modules", "Packages and Modules"),
    (8, "classes", "Classes"),
    (9, "interfaces", "Interfaces"),
    (10, "arrays", "Arrays"),
    (11, "exceptions", "Exceptions"),
    (12, "execution", "Execution"),
    (13, "binary-compatibility", "Binary Compatibility"),
    (14, "blocks-statements-patterns", "Blocks, Statements, and Patterns"),
    (15, "expressions", "Expressions"),
    (16, "definite-assignment", "Definite Assignment"),
    (17, "threads-locks", "Threads and Locks"),
    (18, "type-inference", "Type Inference"),
    (19, "syntax", "Syntax"),
]

README_HEADER = f"""\
# Java Language Specification (SE 26)

**Authors:** James Gosling, Bill Joy, Guy Steele, Gilad Bracha, Alex Buckley, Daniel Smith, Gavin Bierman
**Date:** 2026-02-17
**Source:** {BASE_URL}/index.html
"""

if __name__ == "__main__":
    print("=== Downloading Java Language Specification (SE 26) ===")
    download_spec("jls", EDITION, BASE_URL, CHAPTERS, README_HEADER)
