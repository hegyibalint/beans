"""Tests for extracting the published LSP specification, without network access."""

import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import lsp
import update


class SpecificationTests(unittest.TestCase):
    def test_update_runs_the_lsp_fetcher_after_the_java_fetchers(self):
        calls = []
        with patch.object(update.jls, "main", side_effect=lambda: calls.append("jls")), \
             patch.object(update.jvms, "main", side_effect=lambda: calls.append("jvms")), \
             patch.object(update.lsp, "main", side_effect=lambda: calls.append("lsp")):
            update.main()

        self.assertEqual(calls, ["jls", "jvms", "lsp"])

    def test_only_the_rendered_specification_is_converted(self):
        page = """<nav>Site navigation</nav>
<div id="markdown-content-container">
<h4><a href="#textDocument_declaration" name="textDocument_declaration">Goto Declaration</a></h4>
<div class="highlight"><pre><code><span class="k">type</span> Declaration = Location;</code></pre></div>
</div>
<footer>Site footer</footer>"""

        html = lsp.specification_html(page)

        self.assertIn('name="textDocument_declaration"', html)
        self.assertIn("<pre><code>type Declaration = Location;</code></pre>", html)
        self.assertNotIn("Site navigation", html)
        self.assertNotIn("Site footer", html)
        self.assertNotIn('class="highlight"', html)

    def test_missing_specification_fails_instead_of_writing_a_shell_page(self):
        with self.assertRaisesRegex(ValueError, "rendered LSP specification"):
            lsp.specification_html("<html><nav>Only navigation</nav></html>")

    def test_empty_specification_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "specification is empty"):
            lsp.specification_html('<div id="markdown-content-container"></div>')

    def test_meta_model_links_point_to_the_pinned_online_artifacts(self):
        page = '<div id="markdown-content-container">specification</div>'
        with patch.object(lsp, "to_markdown", return_value="[model](../metaModel/metaModel.json)"):
            markdown = lsp.convert(page)

        self.assertEqual(
            markdown,
            "[model](https://microsoft.github.io/language-server-protocol/"
            "specifications/lsp/3.17/metaModel/metaModel.json)",
        )


if __name__ == "__main__":
    unittest.main()
