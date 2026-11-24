"""Host-owned workspace, ingestion and fixed workflows. No arbitrary commands."""
import base64
import hashlib
import pathlib
import tempfile

def obj(properties):
    return {'type':'object','properties':properties,'required':list(properties),'additionalProperties':False}

PATH={"type":"string","minLength":1,"maxLength":240}
ID={"type":"string","pattern":"^[0-9a-f]{32}$","maxLength":32}

def validate_input_office(path):
    from design_runner import validate_office
    validate_office(path)
    if path.suffix.lower()=='.xlsx':
        import zipfile,xml.etree.ElementTree as ET
        # The literal-cell generator does not need formulas. Import/preview must
        # not let LibreOffice evaluate network or dynamic data formulas.
        with zipfile.ZipFile(path) as archive:
            for name in archive.namelist():
                if name.startswith('xl/worksheets/') and name.endswith('.xml'):
                    if any(e.tag.rsplit('}',1)[-1]=='f' for e in ET.fromstring(archive.read(name)).iter()):
                        raise ValueError('Formula workbooks require a separate reviewed evaluation path')

SCHEMAS={
    'workspace.metadata':obj({'path':PATH}),
    'workspace.sha256':obj({'path':PATH}),
    'workspace.list':obj({'path':PATH}),
    'artifact.import':obj({'path':PATH}),
    'knowledge.ingest':obj({'path':PATH,'title':{'type':'string','minLength':1,'maxLength':300}}),
    'office.preview':obj({'artifact_id':ID}),
    'artifact.compare':obj({'first_id':ID,'second_id':ID}),
    'workflow.run':obj({'template':{'type':'string','enum':['pdf_preview']},'inputs':obj({'title':{'type':'string','minLength':1,'maxLength':100},'body':{'type':'string','maxLength':30000}})})
}

class HostAdapter:
    def __init__(self,delegate,store,knowledge,workspace,soffice,cancel):
        self.delegate=delegate; self.store=store; self.knowledge=knowledge
        self.workspace=pathlib.Path(workspace); self.soffice=soffice; self.cancel=cancel
        self.input_bytes=None

    def call(self,name,args,previous=None):
        if name=='web.fetch':
            from research_adapter import parse_url
            self.research._hosts=frozenset({parse_url(args['url']).hostname})
            try: return self.delegate.call(name,args,previous=previous)
            finally: self.research._hosts=frozenset()
        if name not in SCHEMAS: return self.delegate.call(name,args,previous=previous)
        from dispatch import validate
        validate(args,SCHEMAS[name])
        if previous is not None: raise ValueError('Unexpected previous revision')
        if self.cancel.is_set(): raise ValueError('Cancelled')
        if name.startswith('workspace.'):
            from workspace_adapter import inspect
            return {'ok':True,'result':inspect(self.workspace,args['path'],name.split('.')[1])}
        if name=='workflow.run':
            from workflow import run
            from workflow_templates import get_template
            result=run(self.delegate,get_template(args['template']),args['inputs'],cancel=self.cancel)
            return {'ok':result['status']=='passed','result':result}
        if name=='office.preview':
            return {'ok':True,'result':self.preview(args['artifact_id'])}
        if name=='artifact.compare':
            import io,difflib,zipfile,xml.etree.ElementTree as ET
            first=self.store.metadata(args['first_id']);second=self.store.metadata(args['second_id'])
            result={'first':first,'second':second,'same_bytes':first['sha256']==second['sha256'],'byte_delta':second['bytes']-first['bytes']}
            def text(record):
                data=base64.b64decode(self.store.retrieve(record['artifact_id']),validate=True)
                if record['extension'] in ('docx','xlsx','pptx'):
                    from design_runner import validate_office
                    with tempfile.TemporaryDirectory() as folder:
                        path=pathlib.Path(folder)/('input.'+record['extension']);path.write_bytes(data);validate_office(path)
                    with zipfile.ZipFile(io.BytesIO(data)) as archive:
                        values=[]
                        for name in sorted(archive.namelist()):
                            if name.endswith('.xml') and name.startswith(('word/document','ppt/slides/slide','xl/sharedStrings','xl/worksheets/sheet')):
                                values.extend(e.text or '' for e in ET.fromstring(archive.read(name)).iter() if e.tag.rsplit('}',1)[-1] in ('t','v'))
                        return '\n'.join(values)[:30000]
                if record['extension']=='pdf':
                    from pypdf import PdfReader
                    return '\n'.join(p.extract_text() or '' for p in PdfReader(io.BytesIO(data)).pages)[:30000]
                return ''
            a,b=text(first),text(second)
            result['text_diff']=''.join(difflib.unified_diff(a.splitlines(True),b.splitlines(True),fromfile='previous',tofile='current'))[:8000]
            result['limitation']='Bounded text and byte comparison; inspect rendered pages for visual changes.'
            return {'ok':True,'result':result}
        # Rust supplies the exact bounded bytes reviewed by the operator. Never
        # reopen the model's path after approval or accept bytes from model args.
        if self.input_bytes is None: raise PermissionError('Reviewed input snapshot required')
        if name=='knowledge.ingest':
            suffix=pathlib.Path(args['path']).suffix.lower()
            if suffix=='.pdf':
                from dispatch import dispatch
                encoded=base64.b64encode(self.input_bytes).decode('ascii')
                info=dispatch('pdf.inspect',{'content':encoded},cancel=self.cancel)
                if not info.get('ok'): raise ValueError('PDF inspection failed')
                extracted=dispatch('pdf.extract_text',{'content':encoded,'pages':list(range(1,info['result']['page_count']+1))},cancel=self.cancel)
                if not extracted.get('ok') or any(p['truncated'] for p in extracted['result']['pages']): raise ValueError('PDF extraction failed or exceeds ingestion limit')
                text='\n'.join(p['text'] for p in extracted['result']['pages'])
            elif suffix=='.docx':
                import io,zipfile,xml.etree.ElementTree as ET
                from design_runner import validate_office
                with tempfile.TemporaryDirectory() as folder:
                    path=pathlib.Path(folder)/'input.docx';path.write_bytes(self.input_bytes);validate_office(path)
                with zipfile.ZipFile(io.BytesIO(self.input_bytes)) as archive:
                    body=ET.fromstring(archive.read('word/document.xml'))
                namespace='{http://schemas.openxmlformats.org/wordprocessingml/2006/main}'
                text='\n'.join(''.join(t.text or '' for t in p.iter(namespace+'t')) for p in body.iter(namespace+'p'))
            else:
                if suffix not in ('.txt','.md','.csv','.json','.rst'): raise ValueError('Supported ingestion: UTF-8 text, DOCX body text, PDF embedded text')
                text=self.input_bytes.decode('utf-8-sig')
            return {'ok':True,'result':{**self.knowledge.ingest(args['path'],args['title'],text,hashlib.sha256(self.input_bytes).hexdigest()),'extraction_scope':'DOCX body paragraphs/tables; PDF embedded text without OCR; complete bounded UTF-8 text'}}
        suffix=pathlib.Path(args['path']).suffix.lower().lstrip('.')
        suffix='jpg' if suffix=='jpeg' else suffix
        if suffix in ('docx','pptx','xlsx'):
            from design_runner import validate_office
            with tempfile.TemporaryDirectory() as folder:
                path=pathlib.Path(folder)/('input.'+suffix); path.write_bytes(self.input_bytes)
                validate_input_office(path)
        envelope={'extension':suffix,'encoding':'base64','bytes':len(self.input_bytes),'content':base64.b64encode(self.input_bytes).decode('ascii')}
        return {'ok':True,'result':self.store.store(envelope)}

    def preview(self,artifact_id):
        if not self.soffice: raise ValueError('Configure reviewed LibreOffice executable first')
        meta=self.store.metadata(artifact_id)
        if meta['extension'] not in ('docx','xlsx','pptx'): raise ValueError('Expected Office artifact')
        data=base64.b64decode(self.store.retrieve(artifact_id),validate=True)
        from design_runner import validate_office
        from execution import run_bounded
        import pypdfium2 as pdfium
        with tempfile.TemporaryDirectory(prefix='vmt-office-preview-') as folder:
            root=pathlib.Path(folder); source=root/('input.'+meta['extension']); source.write_bytes(data)
            validate_input_office(source)
            result=run_bounded([self.soffice,'-env:UserInstallation='+(root/'profile').as_uri(),'--headless','--convert-to','pdf','--outdir',str(root),str(source)],b'',root,90,32000,self.cancel)
            pdf=source.with_suffix('.pdf')
            if result.returncode or not pdf.is_file() or pdf.stat().st_size>500000: raise ValueError('Office conversion failed or exceeds PDF limit')
            raw=pdf.read_bytes()
            pdf_record=self.store.store({'extension':'pdf','encoding':'base64','bytes':len(raw),'content':base64.b64encode(raw).decode('ascii')})
            pages=[]; issues=[]
            with pdfium.PdfDocument(raw) as document:
                if not 1<=len(document)<=40: raise ValueError('Preview supports 1-40 pages')
                for i in range(len(document)):
                    if self.cancel.is_set(): raise ValueError('Cancelled')
                    preview=self.delegate.call('pdf.render',{'artifact_id':pdf_record['artifact_id'],'page':i+1,'dpi':72})
                    if not preview.get('ok'): raise ValueError('Page render failed')
                    page=document[i]
                    try:
                        width,height=page.get_size()
                        textpage=page.get_textpage()
                        try:
                            textpage.count_rects()
                            rectangles=[textpage.get_rect(j) for j in range(min(textpage.count_rects(),1000))]
                        finally: textpage.close()
                    finally: page.close()
                    clipped=sum(left<-.5 or bottom<-.5 or right>width+.5 or top>height+.5 for left,bottom,right,top in rectangles)
                    if clipped: issues.append({'page':i+1,'type':'text_outside_page','rectangles':clipped})
                    pages.append({'page':i+1,'artifact':preview['result'],'text_rectangles':len(rectangles),'outside_page':clipped})
            return {'source_artifact_id':artifact_id,'source_sha256':meta['sha256'],'pdf':pdf_record,'pages':pages,'structural_status':'FAIL' if issues else 'PASS','issues':issues,'visual_review':'PENDING'}
