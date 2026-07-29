#!/usr/bin/env python3
"""Download and convert all pinned Java specifications to Markdown."""

import jls
import jvms


def main():
    for spec in (jls, jvms):
        spec.main()


if __name__ == "__main__":
    main()
