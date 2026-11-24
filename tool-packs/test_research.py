import unittest
from unittest.mock import patch
from research_adapter import parse_url,public_address,extract,ResearchHost,fetch
from policy import PolicyHost

class Tests(unittest.TestCase):
    def test_urls(self):
        for url in ("http://example.com","https://127.0.0.1","https://user@example.com","https://example.com:444","https://example.com/#fragment","https://example.com\\@evil.com"):
            with self.subTest(url=url), self.assertRaises(ValueError): parse_url(url)
        self.assertEqual(parse_url("https://example.com/a").hostname,"example.com")
    def test_dns(self):
        with patch("research_adapter.socket.getaddrinfo",return_value=[(0,0,0,"",("127.0.0.1",443))]):
            with self.assertRaises(PermissionError): public_address("example.com")
        with patch("research_adapter.socket.getaddrinfo",return_value=[(0,0,0,"",("93.184.216.34",443)),(0,0,0,"",("10.0.0.1",443))]):
            with self.assertRaises(PermissionError): public_address("example.com")
    def test_extract(self):
        r=extract("<title>News</title><p>Hello world</p><script>hidden</script><a href='/page'>Next</a><a href='http://bad.com'>Bad</a>","https://example.com/")
        self.assertEqual(r["title"],"News")
        self.assertNotIn("hidden",r["text"])
        self.assertEqual(r["links"],["https://example.com/page"])
        self.assertEqual(r["trust"],"untrusted_external_content")
    def test_host_deny(self):
        with patch("research_adapter.socket.getaddrinfo") as dns:
            with self.assertRaises(PermissionError): fetch("https://example.com",set())
            dns.assert_not_called()
    def test_review(self):
        with self.assertRaises(ValueError): PolicyHost(ResearchHost(None,{"example.com"}),{"web.fetch"})
        host=PolicyHost(ResearchHost(None,{"example.com"}),{"web.fetch"},{"web.fetch"})
        args={"url":"https://example.com"}
        with patch("research_adapter.fetch",return_value={"text":"Example"}) as f:
            with self.assertRaises(PermissionError): host.call("web.fetch",args)
            f.assert_not_called()
            self.assertTrue(host.call("web.fetch",args,approval=host.approve("web.fetch",args))["ok"])
            f.assert_called_once()
