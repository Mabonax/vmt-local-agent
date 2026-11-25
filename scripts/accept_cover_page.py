"""Actual agent/broker acceptance for one-review existing-CV cover production."""
import argparse,hashlib,json,pathlib,shutil,subprocess,sys,zipfile
from lxml import etree as ET

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--checker',required=True);parser.add_argument('--source',required=True);parser.add_argument('--root',required=True);parser.add_argument('--office',required=True)
    args=parser.parse_args();root=pathlib.Path(args.root).resolve();checker=str(pathlib.Path(args.checker).resolve());source=pathlib.Path(args.source).resolve()
    if '.local' not in root.parts or root.exists():raise ValueError('Select a NEW isolated .local acceptance directory')
    root.mkdir(parents=True);workspace=root/'project';workspace.mkdir();profile=root/'profile';base=['--data-dir',str(profile)]
    original=source.read_bytes();digest=hashlib.sha256(original).hexdigest();shutil.copyfile(source,workspace/'CV_Template.docx')
    request=root/'request.txt';request.write_text('add cover page to this cv_template',encoding='utf-8')
    def run(*argv):
        p=subprocess.run([checker,*map(str,argv),*base],capture_output=True,text=True,encoding='utf-8',timeout=210)
        if p.returncode:raise AssertionError(p.stderr+p.stdout)
        return p.stdout
    run('--tool-pack-configure',sys.executable,args.office)
    records=[]
    for name,flags in [('denied',['--deny']),('approved',[]),('repeat',[])]:
        run('--unified-live','qwen3.5:4b',workspace,request,*flags)
        report=json.loads((workspace/'model-trial.json').read_text());(root/f'{name}.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
        assert report['planner']=='fixed-document-cover' and not report['errors']
        if name=='denied':
            assert report['denials']==1 and report['approvals']==0
            assert not (workspace/'CV_Template_With_Cover.docx').exists()
        else:
            assert report['approvals']==1 and report['denials']==0
            expected=workspace/('CV_Template_With_Cover.docx' if name=='approved' else 'CV_Template_With_Cover_2.docx')
            assert expected.is_file()
            complete=[event for event in report['activity'] if event['state']=='Complete']
            assert len(complete)==1 and complete[0]['tool']=='pack__document_add_cover'
            result=json.loads(complete[0]['detail'])['result']
            assert result['saved_to_workspace'] and result['preservation']['original_body_preserved'] and result['preservation']['unchanged_package_parts']
            assert result['preview']['structural_status']=='PASS' and result['preview']['pages']
            assert hashlib.sha256(expected.read_bytes()).hexdigest()==result['sha256']
            with zipfile.ZipFile(source) as old,zipfile.ZipFile(expected) as new:
                for part in old.namelist():
                    if part!='word/document.xml':assert old.read(part)==new.read(part),part
                ns={'w':'http://schemas.openxmlformats.org/wordprocessingml/2006/main'}
                a=ET.fromstring(old.read('word/document.xml')).find('w:body',ns)
                b=ET.fromstring(new.read('word/document.xml')).find('w:body',ns)
                assert [ET.tostring(node) for node in a]==[ET.tostring(node) for node in list(b)[-len(a):]]
            if name=='approved':
                exports=root/'review';exports.mkdir()
                for page in result['preview']['pages']:
                    run('--tool-pack-save',workspace,page['artifact']['artifact_id'],exports/f'page-{page["page"]}.png')
            records.append({'case':name,'approvals':report['approvals'],'seconds':report['seconds'],'pages':len(result['preview']['pages']),'source_preserved':True,'saved_file':str(expected)})
        assert hashlib.sha256((workspace/'CV_Template.docx').read_bytes()).hexdigest()==digest
    assert source.read_bytes()==original
    (root/'acceptance.json').write_text(json.dumps({'cases':records,'denial':'PASS','source_sha256':digest,'status':'PASS'},indent=2))
    print(json.dumps({'status':'PASS','cases':records,'denial':'PASS'}))

if __name__=='__main__':main()
