"""Tests for extracting the JSON-RPC specification without network access."""

import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import jsonrpc


class SpecificationTests(unittest.TestCase):
    def test_only_the_specification_is_extracted_with_examples_and_copyright(self):
        page = """<nav>Site navigation</nav>
<div class="span3">Sidebar</div>
<div class="span9">
<div class="page-header"><h1>JSON-RPC 2.0 Specification</h1></div>
<div><h2 id="notification">4.1 Notification</h2>
<pre>{"jsonrpc": "2.0", "method": "update"}</pre></div>
<div><p>Copyright (C) 2007-2010 by the JSON-RPC Working Group</p>
<p>This document itself may not be modified in any way.</p></div>
</div>
<footer>Site footer</footer>"""

        html = jsonrpc.specification_html(page)

        self.assertIn("JSON-RPC 2.0 Specification", html)
        self.assertIn('<a id="notification"></a>', html)
        self.assertIn('<pre><code>{"jsonrpc": "2.0", "method": "update"}</code></pre>', html)
        self.assertIn("Copyright (C) 2007-2010", html)
        self.assertIn("This document itself may not be modified in any way.", html)
        self.assertNotIn("Site navigation", html)
        self.assertNotIn("Sidebar", html)
        self.assertNotIn("Site footer", html)
        self.assertNotIn("<div", html)

    def test_examples_preserve_code_formatting_and_line_breaks(self):
        page = '<div class="span9"><pre>--&gt; []<br>&lt;--&nbsp;{}</pre></div>'

        html = jsonrpc.specification_html(page)

        self.assertEqual(html, "<pre><code>--&gt; []\n&lt;-- {}</code></pre>")

    def test_missing_empty_and_unclosed_specifications_are_rejected(self):
        for page in [
            "<html><nav>Only navigation</nav></html>",
            '<div class="span9"></div>',
            '<div class="span9"><p>Truncated specification</p>',
        ]:
            with self.subTest(page=page):
                with self.assertRaisesRegex(ValueError, "JSON-RPC specification"):
                    jsonrpc.specification_html(page)

    def test_extracted_html_is_converted_to_markdown(self):
        page = '<div class="span9"><h1>JSON-RPC 2.0 Specification</h1></div>'
        with patch.object(jsonrpc, "to_markdown", return_value="# JSON-RPC 2.0 Specification\n") as convert:
            markdown = jsonrpc.convert(page)

        convert.assert_called_once_with("<h1>JSON-RPC 2.0 Specification</h1>")
        self.assertEqual(markdown, "# JSON-RPC 2.0 Specification\n")


if __name__ == "__main__":
    unittest.main()
