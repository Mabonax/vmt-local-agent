"""Run trusted repository packs only. Not an OS sandbox."""
import json
import pathlib
import subprocess
import sys
import unittest

ROOT=pathlib.Path(__file__).resolve().parent
from dispatch import dispatch

EXAMPLES={
 "pdf.generate":{"title":"Report","body":"Example"},
 "office.docx":{"title":"Report","sections":[]},
 "office.xlsx":{"headers":["Name"],"rows":[["Example"]]},
 "office.pptx":{"title":"Deck","slides":[]},
 "project.detect":{"paths":["Cargo.toml"]},
 "code.metrics":{"text":"hello"},
 "code.dependencies":{"format":"npm","text":"{}"},
 "python.imports":{"text":"import os"},
 "math.calculate":{"operation":"add","a":2,"b":3},
 "data.describe":{"values":[1,2,3]},
 "data.frequency":{"values":["a","a"]},
 "data.json_validate":{"text":"{}"},
 "text.inspect":{"text":"hello"},
 "text.sha256":{"text":"hello"},
 "text.diff":{"before":"a","after":"b"},
 "csv.profile":{"text":"a,b\n1,2"},
 "python.symbols":{"text":"def f(): pass"},
 "text.find":{"text":"hello","query":"he"},
 "document.outline":{"text":"# Title"},
 "design.contrast":{"foreground":"#000000","background":"#ffffff"},
 "document.html":{"title":"Test","sections":[]},
}
class Acceptance(unittest.TestCase):
    def test_all(self):
        fixture=dispatch("pdf.generate",{"title":"Fixture","body":"Example"})["result"]["content"]
        EXAMPLES.update({"pdf.inspect":{"content":fixture},"pdf.extract_text":{"content":fixture,"pages":[1]},"pdf.split":{"content":fixture,"pages":[1]},"pdf.merge":{"documents":[fixture,fixture]}})
        seen=set()
        for folder in sorted(ROOT.iterdir()):
            if not folder.is_dir() or not (folder/"manifest.json").is_file(): continue
            manifest=json.loads((folder/"manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(manifest["protocol"],"stdin-json-v1")
            self.assertEqual(manifest["entrypoint"],"runner.py")
            subprocess.run([sys.executable,"-B","-m","unittest","discover","-s",str(folder)],check=True,timeout=30)
            for tool in manifest["tools"]:
                name=tool["name"]
                with self.subTest(tool=name):
                    self.assertNotIn(name,seen)
                    seen.add(name)
                    for args,expected in [(EXAMPLES[name],True),({"unexpected":True},False)]:
                        req=json.dumps({"tool":name,"args":args}).encode()
                        p=subprocess.run([sys.executable,"-I",str(folder/"runner.py")],input=req,capture_output=True,timeout=manifest["timeout_seconds"])
                        self.assertLessEqual(len(p.stdout),manifest["max_output_bytes"])
                        self.assertEqual(p.returncode,0 if expected else 1)
                        self.assertEqual(json.loads(p.stdout)["ok"],expected)
        self.assertEqual(seen,set(EXAMPLES))
if __name__=="__main__": unittest.main()
