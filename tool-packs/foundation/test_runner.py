import unittest
from runner import execute

class Tests(unittest.TestCase):
    def test_math(self):
        self.assertEqual(execute("math.calculate", {"operation":"multiply","a":6,"b":7}), {"value":42})
        with self.assertRaises(ZeroDivisionError):
            execute("math.calculate", {"operation":"divide","a":1,"b":0})
    def test_stats(self):
        self.assertEqual(execute("data.describe", {"values":[4,1,3,2]})["median"], 2.5)
    def test_frequency(self):
        self.assertEqual(execute("data.frequency", {"values":["a","b","a"]}), {"counts":{"a":2,"b":1}})
    def test_text(self):
        self.assertEqual(execute("text.inspect", {"text":"hello world"})["words"],2)
    def test_json(self):
        self.assertEqual(execute("data.json_validate", {"text":"{\"a\":1}"}),{"valid":True})
        with self.assertRaises(ValueError):
            execute("data.json_validate", {"text":"{\"a\":1,\"a\":2}"})
    def test_reject(self):
        with self.assertRaises(ValueError):
            execute("unknown", {})
        with self.assertRaises(ValueError):
            execute("data.describe", {"values":[True]})

if __name__ == "__main__":
    unittest.main()
