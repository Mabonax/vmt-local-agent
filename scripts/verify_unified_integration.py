"""Synthetic acceptance through the real Rust review/execution bridge.

Run with the configured tool venv. The operator supplies the exact synthetic
workspace/profile and may explicitly enable the example.com HTTPS smoke test.
No model access, package installation, or normal-profile changes occur here.
"""
import argparse,hashlib,json,pathlib,subprocess,time,sys

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--checker',required=True);parser.add_argument('--root',required=True)
    parser.add_argument('--office',required=True);parser.add_argument('--live-https',action='store_true')
    args=parser.parse_args();root=pathlib.Path(args.root).resolve()
    if '.local' not in root.parts or root.exists():raise ValueError('Select a NEW .local acceptance directory')
    root.mkdir(parents=True);workspace=root/'project';workspace.mkdir();profile=root/'profile';exports=root/'exports';exports.mkdir()
    checker=str(pathlib.Path(args.checker).resolve());base=['--data-dir',str(profile)]
    records=[];counter=0
    def run(*argv,success=True):
        result=subprocess.run([checker,*map(str,argv),*base],capture_output=True,text=True,encoding='utf-8',timeout=135)
        if (result.returncode==0)!=success:raise AssertionError(result.stderr+result.stdout)
        return result.stdout
    run('--tool-pack-configure',sys.executable,args.office)
    def call(tool,arguments,previous=None,success=True,internet=False,batch=False):
        nonlocal counter
        counter+=1;request={'tool':tool,'arguments':arguments}
        if previous:request['previous']=previous
        path=root/f'request-{counter:02}.json';path.write_text(json.dumps(arguments if batch else request),encoding='utf-8')
        output=run('--tool-pack-batch' if batch else '--tool-pack-run',workspace,path,*(['--internet'] if internet else []),success=success)
        if not success:return output
        value=json.loads(output);(root/f'result-{counter:02}.json').write_text(json.dumps(value,indent=2),encoding='utf-8');return value if batch else value['result']
    def save(record,name):
        path=exports/name;run('--tool-pack-save',workspace,record['artifact_id'],path)
        assert hashlib.sha256(path.read_bytes()).hexdigest()==record['sha256'];return path
    def case(name,fn):
        try:
            detail=fn();records.append({'case':name,'status':'PASS','detail':detail});print(name,'PASS',flush=True)
        except Exception as error:
            records.append({'case':name,'status':'FAIL','error':str(error)});print(name,'FAIL',str(error)[:500],flush=True)
        (root/'report.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
    artifacts={}
    def word():
        from docx import Document
        first=call('office.docx',{'title':'Orion Operations Report','sections':[{'heading':'Executive Summary','body':'Approve the synthetic Orion pilot budget of R375000. The pilot runs for twelve weeks.'},{'heading':'Delivery and Governance','body':'The Technical Specialist owns delivery. Review progress every Friday and retain evidence for each milestone.'},{'heading':'Risks and Decisions','body':'Limited hardware capacity is the primary risk. Use bounded local inference and review generated documents before release.'}]})
        path=save(first,'Orion_Report.docx');doc=Document(path)
        assert len([p for p in doc.paragraphs if p.style.name=='Heading 1'])==3
        assert '375000' in '\n'.join(p.text for p in doc.paragraphs)
        second=call('office.docx',{'title':'Orion Operations Report Revision','sections':[{'heading':'Updated Decision','body':'Approved synthetic budget is R390000 after the revised delivery estimate.'}]},previous=first['artifact_id'])
        assert second['version']==2 and second['previous']==first['artifact_id']
        comparison=call('artifact.compare',{'first_id':first['artifact_id'],'second_id':second['artifact_id']})
        assert not comparison['same_bytes'] and '390000' in comparison['text_diff']
        artifacts['docx']=first;return {'artifact':first,'revised':second,'headings':3,'model':'SEPARATE'}
    case('A deterministic Word and revision',word)
    def presentation():
        from pptx import Presentation
        record=call('office.pptx',{'title':'Orion Pilot Review','slides':[{'heading':'Decision and Scope','body':'Approve the twelve week pilot\nSynthetic budget R375000\nTechnical Specialist accountable'},{'heading':'Delivery Milestones','body':'Weeks 1 to 4 establish the baseline\nWeeks 5 to 8 demonstrate workflows\nWeeks 9 to 12 verify and hand over'},{'heading':'Controls and Next Steps','body':'Review permissions before execution\nCheck every rendered document\nRecord and resolve failed acceptance checks'}]})
        prs=Presentation(save(record,'Orion_Review.pptx'));assert len(prs.slides)==4
        assert all(len(slide.shapes)>0 for slide in prs.slides);artifacts['pptx']=record
        return {'artifact':record,'editable_slides':4}
    case('B PowerPoint',presentation)
    def workbook():
        from openpyxl import load_workbook
        record=call('office.xlsx',{'headers':['Category','Budget ZAR','Actual ZAR','Variance ZAR'],'rows':[['People',180000,175000,5000],['Equipment',120000,125000,-5000],['Operations',75000,65000,10000],['Total',375000,365000,10000]]})
        book=load_workbook(save(record,'Orion_Budget.xlsx'));sheet=book.active
        assert sheet['B5'].value==375000 and sheet['C5'].value==365000
        assert sheet.freeze_panes=='A2' and sheet['A1'].font.bold
        assert sum(sheet.cell(i,2).value for i in range(2,5))==sheet['B5'].value
        artifacts['xlsx']=record;return {'artifact':record,'literal_totals_verified':True,'formulas':'not generated'}
    case('C Excel',workbook)
    def workflow():
        from pypdf import PdfReader
        result=call('workflow.run',{'template':'pdf_preview','inputs':{'title':'Orion Acceptance Record','body':'The local agent generated, inspected, rendered and hashed this synthetic PDF. Every workflow gate must pass before this case succeeds.'}})
        (root/'workflow.json').write_text(json.dumps(result,indent=2))
        # Workflow results retain outputs separately from step audit records.
        out=result['outputs'];doc=out['document'];preview=out['preview']
        pdf=PdfReader(save(doc,'Orion_Acceptance.pdf'));assert len(pdf.pages)==1
        assert 'synthetic PDF' in pdf.pages[0].extract_text()
        save(preview,'Orion_Acceptance.png');artifacts['pdf']=doc
        return result
    case('D PDF workflow',workflow)
    def knowledge():
        (workspace/'sample.txt').write_text('Orion pilot budget is R375000. The programme lasts twelve weeks. The Technical Specialist owns delivery.',encoding='utf-8')
        doc=call('knowledge.ingest',{'path':'sample.txt','title':'Orion synthetic policy'})
        # Each call starts a new checker/worker process: this exercises durable FTS.
        hits=call('knowledge.search',{'query':'Orion budget','limit':3})['passages']
        assert hits and hits[0]['document_id']==doc['document_id'] and '375000' in hits[0]['excerpt']
        cite=call('knowledge.citation',{'document_id':doc['document_id'],'start':hits[0]['start'],'end':hits[0]['end']})
        assert cite['source_sha256']==hashlib.sha256((workspace/'sample.txt').read_bytes()).hexdigest()
        assert call('knowledge.statistics',{})['documents']==1
        return {'document':doc,'citation':cite,'fresh_process_retrieval':True}
    case('E knowledge persistence and citation',knowledge)
    def security():
        call('math.calculate',{'operation':'add','a':1,'b':2,'approve':True},success=False)
        call('artifact.import',{'path':'../outside.docx'},success=False)
        call('web.fetch',{'url':'https://example.com/'},success=False)
        call('web.fetch',{'url':'https://127.0.0.1/'},success=False,internet=True)
        call('pdf.render',{'artifact_id':'a'*32,'page':1,'dpi':72,'content':'raw'},success=False)
        return {'invalid_schema':True,'traversal':True,'disabled_network':True,'private_destination':True,'raw_artifact_envelope':True}
    case('G broker rejection paths',security)
    def preview():
        pages=[]
        for kind in ['docx','pptx','xlsx']:
            result=call('office.preview',{'artifact_id':artifacts[kind]['artifact_id']})
            assert result['structural_status']=='PASS' and result['pages']
            save(result['pdf'],f'Orion_{kind}_preview.pdf')
            for page in result['pages']:save(page['artifact'],f'Orion_{kind}_page_{page["page"]}.png')
            pages.append({'format':kind,'pages':len(result['pages']),'visual_review':'PENDING','result':result})
        return pages
    case('Office all page rendering',preview)
    if args.live_https:
        def research():
            result=call('',[{'tool':'web.fetch','arguments':{'url':'https://example.com/'}},{'tool':'research.sources','arguments':{}},{'tool':'research.find','arguments':{'query':'Example Domain','limit':3}}],internet=True,batch=True)
            assert result[0]['ok'] and result[1]['result']['sources'] and result[2]['result']['passages']
            return result
        case('F live reviewed public HTTPS',research)
    print(json.dumps({'passed':sum(r['status']=='PASS' for r in records),'failed':sum(r['status']=='FAIL' for r in records),'report':str(root/'report.json')}))
    if any(r['status']=='FAIL' for r in records):raise SystemExit(1)

if __name__=='__main__':main()
