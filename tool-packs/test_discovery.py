import json
import unittest
from unittest.mock import patch
from discovery import catalog,discover,match_workflows

class Tests(unittest.TestCase):
    def test_ranking(self):
        allowed=set(catalog())
        r=discover("create Excel spreadsheet workbook",allowed)
        self.assertEqual(r[0]["definition"]["name"],"office.xlsx")
    def test_policy(self):
        self.assertEqual(discover("pdf render",set()),[])
        self.assertEqual(discover("pdf render",{"office.xlsx"}),[])
    def test_budget(self):
        r=discover("pdf render generate merge split extract",set(catalog()),limit=2,budget_bytes=700)
        self.assertLessEqual(len(r),2)
        self.assertLessEqual(len(json.dumps(r,ensure_ascii=False,separators=(",",":")).encode()),700)
        self.assertEqual(discover("pdf render",set(catalog()),budget_bytes=2),[])
    def test_host_schema(self):
        c=catalog()
        p=c["pdf.render"]["definition"]["input_schema"]["properties"]
        self.assertIn("artifact_id",p); self.assertNotIn("content",p)
        self.assertIn("first_id",c["image.compare"]["definition"]["input_schema"]["properties"])
        self.assertIn("artifact_ids",c["pdf.merge"]["definition"]["input_schema"]["properties"])
    def test_missing(self):
        with patch("discovery.importlib.util.find_spec",return_value=None):
            self.assertEqual(discover("office.xlsx",{"office.xlsx"}),[])
            r=discover("office.xlsx",{"office.xlsx"},available_only=False)
            self.assertEqual(r[0]["missing_modules"],["openpyxl"])
    def test_workflow(self):
        self.assertEqual(match_workflows("generate a PDF report",{"pdf_preview"}),["pdf_preview"])
        self.assertEqual(match_workflows("generate a PDF report",set()),[])
        self.assertEqual(match_workflows("extract PDF text",{"pdf_preview"}),[])
