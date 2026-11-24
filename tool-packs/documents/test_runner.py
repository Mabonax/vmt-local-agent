import unittest
from runner import execute
class Tests(unittest.TestCase):
    def test_outline(self):
        self.assertEqual(len(execute("document.outline",{"text":"# Title\n### Jump"})["warnings"]),1)
    def test_contrast(self):
        self.assertEqual(execute("design.contrast",{"foreground":"#000000","background":"#ffffff"})["ratio"],21)
    def test_html(self):
        result=execute("document.html",{"title":"<script>","sections":[{"heading":"A","body":"<img>"}]})["html"]
        self.assertNotIn("<script>",result)
        self.assertIn("&lt;img&gt;",result)
    def test_invalid(self):
        with self.assertRaises(ValueError): execute("design.contrast",{"foreground":"red","background":"#ffffff"})
