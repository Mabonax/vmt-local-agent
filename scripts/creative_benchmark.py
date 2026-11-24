"""Deterministic creative regression fixtures; no local model or user assets required."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import traceback
import xml.etree.ElementTree as ET

import fitz
from PIL import Image, ImageDraw
import design_runner as runner

CASES = [
    ('poster', {'template':'poster','title':'Digital Innovation Summit','subtitle':'Connecting technology and opportunity','footer':'Register today','asset':0}),
    ('social', {'template':'social','title':'Create with purpose','subtitle':'Ideas into impact','footer':'Discover more','asset':0}),
    ('business-card', {'template':'business_card','title':'Example Studio','subtitle':'Creative technology','footer':'hello@example.invalid','asset':0}),
    ('certificate', {'template':'certificate','title':'Certificate of Achievement','subtitle':'Presented to Example Recipient','footer':'Authorised signature'}),
    ('contrast-repair', {'template':'poster','title':'Readable design','subtitle':'Contrast regression fixture','primary':'#EEEEEE'}),
]
NS = {'s':'http://www.w3.org/2000/svg'}

def normalize(value):
    return ' '.join(value.split())

def require(condition, message):
    if not condition:
        raise AssertionError(message)

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def fixture_image():
    image = Image.new('RGB',(320,160),'#087F83')
    draw = ImageDraw.Draw(image)
    draw.rectangle((0,0,159,159),fill='#D14936')
    image.save('asset-0',format='PNG')

def run_case(name, brief, directory):
    directory.mkdir()
    previous = Path.cwd()
    os.chdir(directory)
    try:
        fixture_image()
        Path('brief.json').write_text(json.dumps(brief,indent=2),encoding='utf-8')
        plan = runner.creative_plan(brief)
        Path('plan.json').write_text(json.dumps(plan,indent=2),encoding='utf-8')
        require(plan['status']=='READY_FOR_RENDER_REVIEW','Planner returned unresolved findings')
        selected = plan['candidates'][0]
        scene = selected['scene']
        expected = [normalize(brief[k]) for k in ('title','subtitle','footer') if brief.get(k,'').strip()]
        actual = [normalize(e['text']) for e in scene['elements'] if e['kind']=='text']
        require(actual==expected,'Scene text changed')
        Path('input').write_text(json.dumps(scene),encoding='utf-8')
        assets = int('asset' in brief)
        hashes = {}
        for ext in ('svg','png','pdf'):
            result = runner.run({'args':{'operation':'svg_compose','output':'design.'+ext,'asset_count':assets}})
            path = Path('artifact.'+ext)
            require(path.is_file() and path.stat().st_size>0,'Missing output '+ext)
            require(result['sha256']==digest(path),'Hash mismatch '+ext)
            require(result['visual_review']=='PENDING','Unexpected visual acceptance')
            hashes[ext]=digest(path)
        root = ET.parse('artifact.svg').getroot()
        svg_text = [normalize(' '.join(n.text or '' for n in node.findall('s:tspan',NS))) for node in root.findall('s:text',NS)]
        require(svg_text==expected,'SVG text changed')
        if assets:
            require(len(root.findall('s:image',NS))==1,'Missing embedded image')
        with fitz.open('artifact.pdf') as pdf:
            require(len(pdf)==1,'Unexpected PDF page count')
            page=pdf[0]
            require(abs(page.rect.width-scene['width'])<=1 and abs(page.rect.height-scene['height'])<=1,'PDF dimension mismatch')
            # Some renderer versions outline text. Record extraction separately.
            extracted=normalize(page.get_text())
            extractable=all(s in extracted for s in expected)
        with Image.open('artifact.png') as png:
            require(png.width*png.height<=16_000_000,'Raster pixel limit')
            require(abs(png.width/png.height-scene['width']/scene['height'])<.02,'Raster aspect mismatch')
            require(png.convert('RGB').getextrema()!=tuple((255,255) for _ in range(3)),'Blank white render')
            if assets:
                box=next(e for e in scene['elements'] if e['kind']=='image')
                sx,sy=png.width/scene['width'],png.height/scene['height']
                crop=png.convert('RGB').crop((int(box['x']*sx),int(box['y']*sy),int((box['x']+box['w'])*sx),int((box['y']+box['h'])*sy)))
                sample=crop.resize((32,32))
                require(any(r>130 and r>g*1.3 and r>b*1.3 for r,g,b in sample.get_flattened_data()),'Embedded image red region missing')
        if name=='contrast-repair':
            require(bool(plan['colour_repairs']),'Expected contrast correction absent')
        return {'case':name,'status':'PASS','selected':plan['selected'],'hashes':hashes,
                'pdf_text_extractable':extractable,'visual_review':'PENDING'}
    finally:
        os.chdir(previous)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',required=True,type=Path)
    args=parser.parse_args()
    output=args.output.resolve()
    output.mkdir(parents=True,exist_ok=False)
    results=[]
    for name,brief in CASES:
        try:
            result=run_case(name,brief,output/name)
        except Exception as exc:
            result={'case':name,'status':'FAIL','error':str(exc),'traceback':traceback.format_exc()}
        results.append(result)
        print(name,result['status'],result.get('error',''),flush=True)
    report={'cases':results,'passed':sum(r['status']=='PASS' for r in results),
            'failed':sum(r['status']=='FAIL' for r in results),
            'visual_review':'PENDING','scope':'Deterministic worker fixtures, not live model or aesthetic acceptance'}
    (output/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    return int(bool(report['failed']))

if __name__=='__main__':
    raise SystemExit(main())
