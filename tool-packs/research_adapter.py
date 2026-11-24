"""Reviewed HTTPS retrieval and offline extraction.
Host supplies exact allowed origins. No redirects, cookies, proxies or credentials.
DNS resolution has platform-dependent latency; not a network sandbox.
"""
import hashlib
import http.client
import ipaddress
import re
import socket
import ssl
import time
from html.parser import HTMLParser
from urllib.parse import urlsplit, urljoin

def obj(props):
    return {"type":"object","properties":props,"required":list(props),"additionalProperties":False}
SCHEMAS={
 "web.fetch":obj({"url":{"type":"string","maxLength":2048}}),
 "web.extract":obj({"html":{"type":"string","maxLength":200000},"url":{"type":"string","maxLength":2048}})
}
def parse_url(url):
    if type(url) is not str or len(url)>2048 or not url.isascii() or any(ord(c)<=32 or ord(c)==127 or c==chr(92) for c in url): raise ValueError("Invalid URL")
    p=urlsplit(url)
    if p.scheme!="https" or not p.hostname or p.username is not None or p.password is not None or p.port not in (None,443) or p.fragment: raise ValueError("HTTPS URL required")
    host=p.hostname
    if not re.fullmatch(r"[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?",host) or "." not in host or len(host)>253: raise ValueError("Invalid host")
    try: ipaddress.ip_address(host)
    except ValueError: pass
    else: raise ValueError("IP literals denied")
    if any(not re.fullmatch(r"[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?",label) for label in host.split(".")): raise ValueError("Invalid host")
    return p

def public_address(host):
    addresses=socket.getaddrinfo(host,443,type=socket.SOCK_STREAM)
    ips=[]
    for entry in addresses:
        ip=ipaddress.ip_address(entry[4][0])
        if not ip.is_global or ip.is_multicast or ip.is_reserved or ip.is_loopback or ip.is_link_local or getattr(ip,"ipv4_mapped",None): raise PermissionError("Nonpublic destination denied")
        ips.append(str(ip))
    if not ips: raise ValueError("No address")
    return ips[0]

class PinnedHTTPS(http.client.HTTPSConnection):
    def __init__(self,host,ip):
        super().__init__(host,443,timeout=5,context=ssl.create_default_context())
        self._ip=ip
    def connect(self):
        sock=socket.create_connection((self._ip,443),self.timeout)
        try: self.sock=self._context.wrap_socket(sock,server_hostname=self.host)
        except BaseException:
            sock.close()
            raise

def fetch(url,allowed_hosts):
    p=parse_url(url)
    if p.hostname not in allowed_hosts: raise PermissionError("Host denied")
    ip=public_address(p.hostname)
    c=PinnedHTTPS(p.hostname,ip)
    deadline=time.monotonic()+10
    try:
        target=p.path or "/"
        if p.query: target+="?"+p.query
        c.request("GET",target,headers={"Accept":"text/html, text/plain","Accept-Encoding":"identity","User-Agent":"VMT-Research/1.0","Connection":"close"})
        response=c.getresponse()
        if response.status!=200: raise ValueError("Only HTTP 200 accepted; redirects disabled")
        kind=response.headers.get_content_type()
        if kind not in ("text/html","text/plain"): raise ValueError("Unsupported content type")
        if response.headers.get("Content-Encoding","identity").lower()!="identity": raise ValueError("Encoded response denied")
        buf=bytearray()
        while True:
            if time.monotonic()>deadline: raise TimeoutError("Fetch deadline")
            chunk=response.read1(8192)
            if not chunk: break
            buf.extend(chunk)
            if len(buf)>200000: raise ValueError("Response too large")
        charset=response.headers.get_content_charset() or "utf-8"
        if charset.lower() not in ("utf-8","utf8","ascii","us-ascii","iso-8859-1","windows-1252"): raise ValueError("Unsupported charset")
        text=bytes(buf).decode(charset,errors="replace")
        result=extract(text,url) if kind=="text/html" else {"title":"","text":text[:30000],"links":[],"truncated":len(text)>30000}
        result.update(url=url,sha256=hashlib.sha256(buf).hexdigest(),trust="untrusted_external_content")
        return result
    finally: c.close()

class Extractor(HTMLParser):
    def __init__(self,url):
        super().__init__(convert_charrefs=True)
        self.url=url; self.skip=[]; self.title=False; self.titles=[]; self.parts=[]; self.links=[]
    def handle_starttag(self,tag,attrs):
        if tag in ("script","style","template","noscript","svg"): self.skip.append(tag)
        if self.skip: return
        if tag=="title": self.title=True
        if tag in ("p","div","br","li","h1","h2","h3","tr"): self.parts.append("\n")
        if tag=="a" and len(self.links)<100:
            href=dict(attrs).get("href")
            if href:
                try:
                    url=urljoin(self.url,href)
                    parse_url(url)
                    if url not in self.links: self.links.append(url)
                except ValueError: pass
    def handle_endtag(self,tag):
        if self.skip:
            if tag==self.skip[-1]: self.skip.pop()
            return
        if tag=="title": self.title=False
        if tag in ("p","div","li","h1","h2","h3","tr"): self.parts.append("\n")
    def handle_data(self,data):
        if not self.skip:
            if self.title: self.titles.append(data)
            else: self.parts.append(data)

def extract(html,url):
    parse_url(url)
    if type(html) is not str or len(html)>200000: raise ValueError("Invalid HTML")
    p=Extractor(url); p.feed(html); p.close()
    text="\n".join(x for x in (" ".join(line.split()) for line in "".join(p.parts).splitlines()) if x)
    return {"title":" ".join("".join(p.titles).split())[:300],"text":text[:30000],"links":p.links,"truncated":len(text)>30000,"trust":"untrusted_external_content"}

class ResearchHost:
    def __init__(self,delegate,allowed_hosts):
        self._delegate=delegate
        self._hosts=frozenset(allowed_hosts)
    def call(self,name,args,previous=None):
        if name not in SCHEMAS: return self._delegate.call(name,args,previous=previous)
        if previous is not None or type(args) is not dict or set(args)!=set(SCHEMAS[name]["properties"]): raise ValueError("Invalid research request")
        result=fetch(args["url"],self._hosts) if name=="web.fetch" else extract(args["html"],args["url"])
        return {"ok":True,"result":result}
