#!/usr/bin/env python3
"""Download the Java Virtual Machine Specification and convert it to Markdown.

The edition is pinned; bump it deliberately, the spec-test citations
follow it. Requires pandoc.
"""

from common import download_spec

EDITION = "se26"
BASE_URL = f"https://docs.oracle.com/javase/specs/jvms/{EDITION}/html"

CHAPTERS = [
    (1, "introduction", "Introduction"),
    (2, "jvm-structure", "The Structure of the Java Virtual Machine"),
    (3, "compiling", "Compiling for the Java Virtual Machine"),
    (4, "class-file-format", "The class File Format"),
    (5, "loading-linking-initializing", "Loading, Linking, and Initializing"),
    (6, "instruction-set", "The Java Virtual Machine Instruction Set"),
    (7, "opcode-mnemonics", "Opcode Mnemonics by Opcode"),
]

README_HEADER = f"""\
# Java Virtual Machine Specification (SE 26)

**Authors:** Tim Lindholm, Frank Yellin, Gilad Bracha, Alex Buckley, Daniel Smith
**Date:** 2026-02-17
**Source:** {BASE_URL}/index.html
"""

if __name__ == "__main__":
    print("=== Downloading JVM Specification (SE 26) ===")
    download_spec("jvms", EDITION, BASE_URL, CHAPTERS, README_HEADER)
