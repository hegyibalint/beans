#!/usr/bin/env python3
"""Download and convert the pinned Java and LSP specifications to Markdown."""

import jls
import jvms
import lsp


def main():
    for spec in (jls, jvms, lsp):
        spec.main()


if __name__ == "__main__":
    main()
