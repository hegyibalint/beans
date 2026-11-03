#!/usr/bin/env python3
"""Download and convert the pinned Java, LSP, and JSON-RPC specifications to Markdown."""

import jls
import jvms
import lsp
import jsonrpc


def main():
    for spec in (jls, jvms, lsp, jsonrpc):
        spec.main()


if __name__ == "__main__":
    main()
