import unittest
from runner import execute
class Tests(unittest.TestCase):
    def test_detect(self):
        self.assertEqual(execute("project.detect",{"paths":["Cargo.toml","src/main.rs"]})["detected"],["Rust"])
    def test_metrics(self):
        self.assertEqual(execute("code.metrics",{"text":"a\n\nb"})["nonblank_lines"],2)
    def test_dependencies(self):
        self.assertEqual(execute("code.dependencies",{"format":"cargo","text":'[dependencies]\nserde="1"'})["dependencies"][0]["name"],"serde")
        self.assertEqual(execute("code.dependencies",{"format":"npm","text":'{"dependencies":{"react":"^19"}}'})["dependencies"][0]["name"],"react")
    def test_imports(self):
        self.assertEqual(execute("python.imports",{"text":"from .foo import bar"})["imports"][0]["module"],".foo")
    def test_invalid(self):
        with self.assertRaises(ValueError): execute("code.dependencies",{"format":"npm","text":"[]"})
