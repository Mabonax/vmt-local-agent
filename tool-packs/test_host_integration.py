import base64
import hashlib
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import time
import unittest
from artifact_store import ArtifactStore
from artifact_host import ArtifactHost
from discovery import catalog,discover
from process_limits import WindowsJob
from session_worker import Session

class HostIntegration(unittest.TestCase):
    def test_session_writer_lease_rejects_concurrent_processes_and_releases(self):
        with tempfile.TemporaryDirectory() as folder:
            args={'state_directory':folder,'workspace':folder,'collection':'default','allowed':['math.calculate'],'soffice':None}
            one=Session(args);one.host()
            two=Session(args)
            try:
                with self.assertRaises(PermissionError):two.host()
            finally:two.close();one.close()
            reopened=Session(args);reopened.host();reopened.close()

    def test_failed_workflow_is_not_successful_host_execution(self):
        from host_adapter import HostAdapter
        import threading
        class Reject:
            def call(self,*_,**__):return {'ok':False,'error':'Synthetic failure'}
        adapter=HostAdapter(Reject(),None,None,'.',None,threading.Event())
        response=adapter.call('workflow.run',{'template':'pdf_preview','inputs':{'title':'test','body':'test'}})
        self.assertFalse(response['ok']);self.assertEqual(response['result']['status'],'failed')

    def test_exact_docx_edit_preserves_package(self):
        import io,zipfile
        from docx import Document
        from document_edit import replace_text
        doc=Document();doc.add_paragraph('Budget 100');doc.add_table(rows=1,cols=1).cell(0,0).text='Keep table'
        doc.sections[0].header.paragraphs[0].text='Keep header'
        buf=io.BytesIO();doc.save(buf);source=buf.getvalue()
        raw,proof=replace_text(source,'100','200',1)
        reopened=Document(io.BytesIO(raw))
        self.assertEqual(reopened.paragraphs[0].text,'Budget 200')
        self.assertEqual(reopened.tables[0].cell(0,0).text,'Keep table')
        self.assertEqual(reopened.sections[0].header.paragraphs[0].text,'Keep header')
        self.assertTrue(proof['authorised_text_only'])
        with zipfile.ZipFile(io.BytesIO(source)) as a,zipfile.ZipFile(io.BytesIO(raw)) as b:
            self.assertTrue(all(a.read(n)==b.read(n) for n in a.namelist() if n!='word/document.xml'))
        with self.assertRaises(ValueError):replace_text(source,'100','200',2)
        doc=Document();p=doc.add_paragraph();p.add_run('Bud');p.add_run('get')
        buf=io.BytesIO();doc.save(buf)
        with self.assertRaises(ValueError):replace_text(buf.getvalue(),'Budget','Total',1)

    def test_word_workflow_and_child_allowlist(self):
        with tempfile.TemporaryDirectory() as folder:
            allowed=['workflow.run','office.docx','artifact.inspect','artifact.hash']
            args={'state_directory':folder,'workspace':folder,'collection':'default','allowed':allowed,'soffice':None}
            request={'tool':'workflow.run','arguments':{'template':'word_report','inputs':{'title':'Report','body':'Supplied facts only'}}}
            session=Session(args)
            try:
                self.assertIn('word_report',session.request({'action':'discover','query':'create Word report'})['workflows'])
                token=session.request({'action':'approve',**request})['approval']
                result=session.request({'action':'call',**request,'approval':token})
                self.assertTrue(result['ok'])
                self.assertEqual(result['result']['outputs']['document']['extension'],'docx')
            finally:session.close()
            session=Session({**args,'allowed':['workflow.run']})
            try:
                token=session.request({'action':'approve',**request})['approval']
                with self.assertRaises(PermissionError):
                    session.request({'action':'call',**request,'approval':token})
            finally:session.close()

    def test_generated_catalog_matches_reviewed_source(self):
        path=pathlib.Path(__file__).parent/'host-catalog.json'
        self.assertEqual(json.loads(path.read_text()),{name:item['definition'] for name,item in catalog().items()})

    def test_knowledge_source_revisions_survive_reopen(self):
        from knowledge import KnowledgeStore
        with tempfile.TemporaryDirectory() as folder:
            store=KnowledgeStore(folder)
            first=store.ingest('sample.txt','Sample','Orion budget is 375000.','a'*64)
            second=store.ingest('sample.txt','Sample','Orion budget is 375000.','b'*64)
            self.assertEqual(second['revision'],2);store.close()
            restored=KnowledgeStore(folder)
            self.assertEqual(restored.search('Orion',3)[0]['document_id'],second['document_id'])
            self.assertEqual(restored.search('Orion',3)[0]['source_sha256'],'b'*64)
            restored.close()

    def test_formula_workbooks_cannot_enter_office_preview(self):
        sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent.parent/'scripts'))
        from openpyxl import Workbook
        from host_adapter import validate_input_office
        with tempfile.TemporaryDirectory() as folder:
            path=pathlib.Path(folder)/'formula.xlsx'
            book=Workbook();book.active['A1']='=WEBSERVICE("https://example.com")';book.save(path)
            with self.assertRaises(ValueError):validate_input_office(path)

    def test_durable_artifact_ancestry_and_corruption(self):
        with tempfile.TemporaryDirectory() as folder:
            store=ArtifactStore(folder,persistent=True);host=ArtifactHost(store)
            one=host.call('office.docx',{'title':'Original','sections':[{'heading':'Scope','body':'Preserved test text'}]})['result']
            two=host.call('office.docx',{'title':'Revision','sections':[{'heading':'Scope','body':'Changed test text'}]},previous=one['artifact_id'])['result']
            restored=ArtifactStore(folder,persistent=True)
            self.assertEqual(restored.metadata(two['artifact_id'])['previous'],one['artifact_id'])
            self.assertEqual(restored.metadata(two['artifact_id'])['version'],2)
            data=base64.b64decode(restored.retrieve(one['artifact_id']))
            self.assertEqual(hashlib.sha256(data).hexdigest(),one['sha256'])
            path=pathlib.Path(folder)/'artifacts'/(one['artifact_id']+'.docx');path.write_bytes(b'not a document')
            with self.assertRaises(ValueError):restored.retrieve(one['artifact_id'])

    def test_durable_index_rejects_forged_paths_and_ancestry(self):
        with tempfile.TemporaryDirectory() as folder:
            store=ArtifactStore(folder,persistent=True)
            record={'artifact_id':'../escape','extension':'pdf','bytes':3,'sha256':'0'*64,'previous':None,'version':1}
            (store._root/'index.json').write_text(json.dumps([record]))
            with self.assertRaises(ValueError):ArtifactStore(folder,persistent=True)

    def test_model_catalog_excludes_host_transport_and_binary_envelopes(self):
        entries=catalog()
        self.assertEqual(len(entries),60)
        self.assertFalse({'approve','export','call','init'} & entries.keys())
        for item in entries.values():
            if item['definition']['name'] in ('pdf.inspect','pdf.render','image.inspect'):
                self.assertNotIn('content',item['definition']['input_schema']['properties'])
        selected=discover('Word report',{'office.docx'},budget_bytes=6000)
        self.assertEqual([v['definition']['name'] for v in selected],['office.docx'])

    def test_discovery_filters_unavailable_before_ranking_and_workflow_policy(self):
        with tempfile.TemporaryDirectory() as folder:
            session=Session({'state_directory':folder,'workspace':folder,'collection':'default','allowed':['office.preview','office.docx','document.add_cover','pdf.generate'],'soffice':None})
            result=session.request({'action':'discover','query':'office.preview Word report','limit':1})
            self.assertEqual([x['definition']['name'] for x in result['tools']],['office.docx'])
            self.assertEqual(session.request({'action':'discover','query':'generate PDF report'})['workflows'],[])
            session.close()

    def test_expired_approval_attachments_are_released(self):
        with tempfile.TemporaryDirectory() as folder:
            session=Session({'state_directory':folder,'workspace':folder,'collection':'default','allowed':['math.calculate'],'soffice':None})
            try:
                args={'action':'approve','tool':'math.calculate','arguments':{'operation':'add','a':1,'b':2},'attachment':base64.b64encode(b'test').decode()}
                old=session.request(args)['approval']
                policy=session.host()
                policy._tokens[old]=(policy._tokens[old][0],time.monotonic()-1)
                new=session.request(args)['approval']
                self.assertNotIn(old,session._snapshots)
                self.assertNotIn(old,session._designs)
                self.assertIn(new,session._snapshots)
            finally:session.close()

    def test_real_session_approval_replay_mutation_and_expiry(self):
        with tempfile.TemporaryDirectory() as folder:
            session=Session({'state_directory':folder,'workspace':folder,'collection':'default','allowed':['math.calculate'],'soffice':None})
            args={'operation':'add','a':2,'b':3}
            token=session.request({'action':'approve','tool':'math.calculate','arguments':args})['approval']
            call={'action':'call','tool':'math.calculate','arguments':args,'approval':token}
            self.assertEqual(session.request(call)['result']['value'],5)
            with self.assertRaises(PermissionError):session.request(call)
            token=session.request({'action':'approve','tool':'math.calculate','arguments':args})['approval']
            with self.assertRaises(PermissionError):session.request({**call,'approval':token,'arguments':{**args,'a':99}})
            token=session.request({'action':'approve','tool':'math.calculate','arguments':args})['approval']
            record=session.host()._tokens[token];session.host()._tokens[token]=(record[0],time.monotonic()-1)
            with self.assertRaises(PermissionError):session.request({**call,'approval':token})
            session.close()

    @unittest.skipUnless(os.name=='nt','Windows Job Object verification')
    def test_windows_memory_and_cpu_limits(self):
        script="import sys;sys.stdin.buffer.read(1)\ntry:\n x=bytearray(512*1024*1024);print('unexpected')\nexcept MemoryError: print('limited')"
        process=subprocess.Popen([sys.executable,'-I','-c',script],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        job=WindowsJob(process,memory_bytes=128*1024*1024)
        try:
            stdout,_=process.communicate(b'x',timeout=10)
            self.assertEqual(stdout.strip(),b'limited')
        finally:job.close()
        process=subprocess.Popen([sys.executable,'-I','-c','import sys;sys.stdin.buffer.read(1)\nwhile True: pass'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        job=WindowsJob(process,cpu_seconds=.15)
        try:
            process.communicate(b'x',timeout=10)
            self.assertNotEqual(process.returncode,0)
        finally:job.close()

    @unittest.skipUnless(os.name=='nt','Windows descendant containment')
    def test_windows_job_close_kills_descendants(self):
        import psutil
        script="import sys,subprocess,time;sys.stdin.buffer.read(1);p=subprocess.Popen([sys.executable,'-I','-c','import time;time.sleep(60)']);print(p.pid,flush=True);time.sleep(60)"
        process=subprocess.Popen([sys.executable,'-I','-c',script],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        job=WindowsJob(process)
        process.stdin.write(b'x');process.stdin.close();process.stdin=None
        child=int(process.stdout.readline())
        job.close();process.wait(timeout=5)
        for _ in range(100):
            if not psutil.pid_exists(child):break
            time.sleep(.01)
        self.assertFalse(psutil.pid_exists(child))

if __name__=='__main__':unittest.main()
