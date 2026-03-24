#!/bin/bash
#
# Download JLS and JVMS specifications from Oracle, convert to Markdown.
#
# Sources:
#   JLS:  https://docs.oracle.com/javase/specs/jls/se26/html/
#   JVMS: https://docs.oracle.com/javase/specs/jvms/se26/html/
#
# Requirements: curl, pandoc

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

JLS_BASE="https://docs.oracle.com/javase/specs/jls/se26/html"
JVMS_BASE="https://docs.oracle.com/javase/specs/jvms/se26/html"
JLS_OUT="$PROJECT_DIR/docs/lang-specs/jls"
JVMS_OUT="$PROJECT_DIR/docs/lang-specs/jvms"

# Check dependencies
for cmd in curl pandoc; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "Error: $cmd is required but not found" >&2
        exit 1
    fi
done

mkdir -p "$JLS_OUT" "$JVMS_OUT"

# JLS chapters: number|slug|title
JLS_CHAPTERS=(
    "1|introduction|Introduction"
    "2|grammars|Grammars"
    "3|lexical-structure|Lexical Structure"
    "4|types-values-variables|Types, Values, and Variables"
    "5|conversions-contexts|Conversions and Contexts"
    "6|names|Names"
    "7|packages-modules|Packages and Modules"
    "8|classes|Classes"
    "9|interfaces|Interfaces"
    "10|arrays|Arrays"
    "11|exceptions|Exceptions"
    "12|execution|Execution"
    "13|binary-compatibility|Binary Compatibility"
    "14|blocks-statements-patterns|Blocks, Statements, and Patterns"
    "15|expressions|Expressions"
    "16|definite-assignment|Definite Assignment"
    "17|threads-locks|Threads and Locks"
    "18|type-inference|Type Inference"
    "19|syntax|Syntax"
)

# JVMS chapters: number|slug|title
JVMS_CHAPTERS=(
    "1|introduction|Introduction"
    "2|jvm-structure|The Structure of the Java Virtual Machine"
    "3|compiling|Compiling for the Java Virtual Machine"
    "4|class-file-format|The class File Format"
    "5|loading-linking-initializing|Loading, Linking, and Initializing"
    "6|instruction-set|The Java Virtual Machine Instruction Set"
    "7|opcode-mnemonics|Opcode Mnemonics by Opcode"
)

# Build a sed expression to rewrite cross-chapter links for a given spec.
# Converts e.g. jls-8.html -> ch08-classes.md
build_link_rewrite_sed() {
    local prefix="$1"
    shift
    local chapters=("$@")
    local sed_expr=""

    for entry in "${chapters[@]}"; do
        IFS='|' read -r num slug _title <<< "$entry"
        local padded
        padded=$(printf "%02d" "$num")
        sed_expr="${sed_expr}s|${prefix}-${num}\\.html|ch${padded}-${slug}.md|g;"
    done
    echo "$sed_expr"
}

# Clean pandoc GFM output: strip Oracle nav chrome, raw div tags, fix links
clean_markdown() {
    local prefix="$1"  # "jls" or "jvms"
    local link_rewrite="$2"

    # Pipeline:
    # 1. Remove everything up to and including the first '# Chapter' heading,
    #    then re-emit that heading
    # 2. Remove navfooter and everything after
    # 3. Strip raw HTML div tags (but keep content)
    # 4. Strip <span> tags (pandoc leaves class="trademark", class="emphasis" etc.)
    # 5. Rewrite cross-chapter links
    # 6. Rewrite self-referencing links (e.g. jls-1.html#jls-1.1 -> #jls-1.1)
    # 7. Clean up excessive blank lines

    awk '
        BEGIN { found_heading = 0; in_footer = 0 }
        /^<div class="navfooter">/ { in_footer = 1 }
        in_footer { next }
        /^# Chapter/ { found_heading = 1 }
        found_heading { print }
    ' \
    | sed -E '
        # Remove raw <div> and </div> tags (with optional attributes), even indented
        s/^[[:space:]]*<div[^>]*>//g
        s/<\/div>//g

        # Remove <span> wrappers, keeping content
        s/<span[^>]*>//g
        s/<\/span>//g

        # Remove <em> and </em> tags (flatten before link conversion)
        s/<em>//g
        s/<\/em>//g

        # Clean up trademark symbols left in spans
        s/\s*®//g

        # Remove empty bold markers from TOC
        s/\*\*Table of Contents\*\*/## Contents/

        # Remove accesskey attributes from links
        s/ accesskey="[^"]*"//g
    ' \
    | perl -pe 's/<a href="([^"]*)"[^>]*>(.*?)<\/a>/[$2]($1)/g' \
    | sed "$link_rewrite" \
    | sed -E "s|${prefix}-[0-9]+\\.html#|#|g" \
    | awk '
        # Collapse runs of 3+ blank lines to 2
        /^$/ { blank++; if (blank <= 2) print; next }
        { blank = 0; print }
    '
}

download_and_convert() {
    local base_url="$1"
    local prefix="$2"   # "jls" or "jvms"
    local num="$3"
    local slug="$4"
    local title="$5"
    local out_dir="$6"
    local link_rewrite="$7"

    local padded
    padded=$(printf "%02d" "$num")
    local src="${base_url}/${prefix}-${num}.html"
    local dst="${out_dir}/ch${padded}-${slug}.md"

    echo "  Downloading ${prefix}-${num}: ${title}..."

    curl -s "$src" \
        | pandoc -f html -t gfm --wrap=none \
        | clean_markdown "$prefix" "$link_rewrite" \
        > "$dst"

    local lines
    lines=$(wc -l < "$dst")
    echo "    -> $(basename "$dst") (${lines} lines)"
}

# Build link rewrite sed expressions
JLS_LINK_SED=$(build_link_rewrite_sed "jls" "${JLS_CHAPTERS[@]}")
JVMS_LINK_SED=$(build_link_rewrite_sed "jvms" "${JVMS_CHAPTERS[@]}")

echo "=== Downloading Java Language Specification (SE 26) ==="
for entry in "${JLS_CHAPTERS[@]}"; do
    IFS='|' read -r num slug title <<< "$entry"
    download_and_convert "$JLS_BASE" "jls" "$num" "$slug" "$title" "$JLS_OUT" "$JLS_LINK_SED"
    sleep 0.5
done

echo ""
echo "=== Downloading JVM Specification (SE 26) ==="
for entry in "${JVMS_CHAPTERS[@]}"; do
    IFS='|' read -r num slug title <<< "$entry"
    download_and_convert "$JVMS_BASE" "jvms" "$num" "$slug" "$title" "$JVMS_OUT" "$JVMS_LINK_SED"
    sleep 0.5
done

echo ""
echo "=== Generating README files ==="

# Generate JLS README
cat > "$JLS_OUT/README.md" << 'JLSEOF'
# Java Language Specification (SE 26)

**Authors:** James Gosling, Bill Joy, Guy Steele, Gilad Bracha, Alex Buckley, Daniel Smith, Gavin Bierman
**Date:** 2026-02-17
**Source:** https://docs.oracle.com/javase/specs/jls/se26/html/index.html

## Chapters

1. [Introduction](ch01-introduction.md)
2. [Grammars](ch02-grammars.md)
3. [Lexical Structure](ch03-lexical-structure.md)
4. [Types, Values, and Variables](ch04-types-values-variables.md)
5. [Conversions and Contexts](ch05-conversions-contexts.md)
6. [Names](ch06-names.md)
7. [Packages and Modules](ch07-packages-modules.md)
8. [Classes](ch08-classes.md)
9. [Interfaces](ch09-interfaces.md)
10. [Arrays](ch10-arrays.md)
11. [Exceptions](ch11-exceptions.md)
12. [Execution](ch12-execution.md)
13. [Binary Compatibility](ch13-binary-compatibility.md)
14. [Blocks, Statements, and Patterns](ch14-blocks-statements-patterns.md)
15. [Expressions](ch15-expressions.md)
16. [Definite Assignment](ch16-definite-assignment.md)
17. [Threads and Locks](ch17-threads-locks.md)
18. [Type Inference](ch18-type-inference.md)
19. [Syntax](ch19-syntax.md)
JLSEOF

cat > "$JVMS_OUT/README.md" << 'JVMSEOF'
# Java Virtual Machine Specification (SE 26)

**Authors:** Tim Lindholm, Frank Yellin, Gilad Bracha, Alex Buckley, Daniel Smith
**Date:** 2026-02-17
**Source:** https://docs.oracle.com/javase/specs/jvms/se26/html/index.html

## Chapters

1. [Introduction](ch01-introduction.md)
2. [The Structure of the Java Virtual Machine](ch02-jvm-structure.md)
3. [Compiling for the Java Virtual Machine](ch03-compiling.md)
4. [The class File Format](ch04-class-file-format.md)
5. [Loading, Linking, and Initializing](ch05-loading-linking-initializing.md)
6. [The Java Virtual Machine Instruction Set](ch06-instruction-set.md)
7. [Opcode Mnemonics by Opcode](ch07-opcode-mnemonics.md)
JVMSEOF

echo "  -> jls/README.md"
echo "  -> jvms/README.md"

echo ""
echo "=== Done ==="
echo "JLS:  $(ls "$JLS_OUT"/*.md | wc -l | tr -d ' ') files in $JLS_OUT/"
echo "JVMS: $(ls "$JVMS_OUT"/*.md | wc -l | tr -d ' ') files in $JVMS_OUT/"
