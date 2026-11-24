import base64
import io
import unittest
from PIL import Image
from reportlab.pdfgen import canvas
from runner import execute

def png(color,size=(10,10)):
    b=io.BytesIO(); Image.new("RGB",size,color).save(b,format="PNG")
    return base64.b64encode(b.getvalue()).decode()

class Tests(unittest.TestCase):
    def test_render(self):
        b=io.BytesIO(); c=canvas.Canvas(b,pagesize=(200,300)); c.drawString(20,250,"Hello"); c.save()
        data=base64.b64encode(b.getvalue()).decode()
        result=execute("pdf.render",{"content":data,"page":1,"dpi":72})
        stats=execute("image.inspect",{"content":result["content"]})
        self.assertEqual((stats["width"],stats["height"]),(200,300))
        self.assertGreater(stats["nonwhite_fraction"],0)
        with self.assertRaises(ValueError): execute("pdf.render",{"content":data,"page":2,"dpi":72})
    def test_blank(self):
        r=execute("image.inspect",{"content":png("white")})
        self.assertEqual(r["nonwhite_fraction"],0)
        self.assertIsNone(r["content_bounds"])
    def test_compare(self):
        a=png("white"); b=png("black")
        self.assertEqual(execute("image.compare",{"first":a,"second":a})["changed_fraction"],0)
        self.assertEqual(execute("image.compare",{"first":a,"second":b})["changed_fraction"],1)
        with self.assertRaises(ValueError): execute("image.compare",{"first":a,"second":png("black",(11,10))})
