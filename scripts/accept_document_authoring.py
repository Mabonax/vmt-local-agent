"""Set up and independently verify a synthetic real-model document trial.

This script writes a brief, never the model's content JSON or Word document.
The Rust --document-authoring check runs the actual agent and review broker.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re

HEADINGS = ['Executive Summary', 'Objectives and Scope', 'Delivery Plan',
            'Technical Architecture', 'Staffing and Governance', 'Budget',
            'Risk Register', 'Training and Adoption', 'Monitoring and Evaluation',
            'Handover and Acceptance']
BRIEF = '''Create an original, substantial Word proposal from scratch for a fictional single-site community technology lab. This is a synthetic test, not a real funding application. Draft the prose yourself from these facts; there is no prepared document or content JSON.
Title: Community Technology Lab Implementation Proposal.
Plan a 12-week pilot serving 30 learners with 20 workstations. Use at least 900 words of useful prose and bullets in total, excluding tables; avoid repetitive filler. Include these ten section headings in this order: Executive Summary; Objectives and Scope; Delivery Plan; Technical Architecture; Staffing and Governance; Budget; Risk Register; Training and Adoption; Monitoring and Evaluation; Handover and Acceptance.
Include three editable native Word tables: Delivery Plan has a header plus four phases with weeks, deliverable and accountable owner; Risk Register has a header plus four concrete risks with impact and mitigation; Budget has the following exact cells, treating all amounts as synthetic ZAR values:
Item | Quantity | Unit cost (ZAR) | Total (ZAR)
Workstations | 20 | 12,000 | 240,000
Network | 1 | 35,000 | 35,000
Training | 1 | 60,000 | 60,000
Contingency | 1 | 25,000 | 25,000
Total | | | 360,000
Explain responsibilities, offline operation and backups, learner support, measurable outcomes and specific handover acceptance criteria. Do not invent real approvals, real suppliers, citations or people. Use coherent headings, readable typography and restrained navy/teal colour. Preserve all numerical facts.
Save the model-authored editable content source as proposal.json, create proposal-v1.docx using the installed document skill and render every page into review-v1. Load the relevant installed document skill and its documented contract before composing. You may draft the content source incrementally with write_file and edit_file if it is too long for one response. Only proposal.json may be written or edited by file tools. Use dedicated composition/render tools; no shell commands, workflows, network, MCP or memory writes. Inspect actual results and report any failure or remaining visual review honestly. A message alone is not completion.
'''
ADD = '''Add a new section to the existing proposal-v1.docx using its model-authored editable source proposal.json. Read the document and source first. Preserve every existing section, paragraph, bullet, table cell, title and design token without rewriting. Append exactly one section headed Appendix - Incident Response, with at least 120 new words describing detection, triage, containment and recovery responsibilities. Add a new two-column native Word table with header Priority | Response target and rows P1 | 30 minutes and P2 | 4 hours. Keep all ten original sections and the budget unchanged.
Update proposal.json, create a separate proposal-v2.docx, and render every page into review-v2. Preserve proposal-v1.docx byte-for-byte. Load the document skill/contract you need. Use only proposal.json file edits and the dedicated document tools; no shell, workflows, network, MCP or memory writes. Report whether you edited the native package or rebuilt it from its editable content source. Never claim visual review just from rendering.
'''
EDIT = '''Edit the existing proposal-v2.docx using proposal.json. Read the current document and source first. Make only these exact changes: replace every occurrence of 60,000 with 75,000, and every occurrence of 360,000 with 375,000, including the Training budget row, Total row and any prose. The Training quantity stays 1. Keep all other wording, sections, table cells, numeric facts, appendix and design tokens unchanged. The resulting budget must still add up.
Update proposal.json, create a separate proposal-v3.docx and render every page into review-v3. Preserve both earlier DOCX versions byte-for-byte. Use only proposal.json file edits and the dedicated document tools; load the installed document contract as needed. No shell, workflows, network, MCP or memory writes. Report whether you edited the native package or rebuilt it from the editable content source. Be honest about failures and visual review limitations.
'''

def save(path, value):
    path.write_text(json.dumps(value, indent=2), encoding='utf-8')

def setup(root):
    root.mkdir(parents=True, exist_ok=False)
    (root / 'project').mkdir()
    for stage, request in enumerate((BRIEF, ADD, EDIT), 1):
        (root / f'request-{stage}.txt').write_text(request, encoding='utf-8')
    (root / 'project' / 'brief.txt').write_text(BRIEF, encoding='utf-8')
    save(root / 'criteria.json', {'minimum_words_excluding_tables':900, 'minimum_pages':3,
        'headings':HEADINGS, 'native_tables':3, 'addition_minimum_words':120,
        'budget_total_before':360000, 'budget_total_after':375000})
    print(root.resolve())

def norm(text):
    return ' '.join(text.split())

def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)
    elif isinstance(value, dict):
        for key, item in value.items():
            if key in ('title', 'subtitle', 'sections', 'heading', 'paragraphs', 'bullets', 'table'):
                yield from strings(item)

def changed(value):
    if isinstance(value, str):
        return value.replace('60,000', '75,000').replace('360,000', '375,000')
    if isinstance(value, list):
        return [changed(v) for v in value]
    if isinstance(value, dict):
        return {k:changed(v) for k,v in value.items()}
    return value

def verify(root, stage):
    from docx import Document
    checks = {}
    errors = []
    try:
        runtime = json.loads((root / f'stage-{stage}-report.json').read_text(encoding='utf-8'))
        checks['runtime_completed'] = runtime['runtime_passed']
        source = json.loads((root / f'proposal-source-v{stage}.json').read_text(encoding='utf-8-sig'))
        document = Document(root / 'project' / f'proposal-v{stage}.docx')
        paragraphs = [p.text for p in document.paragraphs]
        tables = [[[c.text for c in r.cells] for r in t.rows] for t in document.tables]
        actual = norm('\n'.join(paragraphs + [c for t in tables for r in t for c in r]))
        checks['all_authored_text_in_docx'] = all(norm(s) in actual for s in strings(source))
        checks['all_authored_native_tables_match'] = tables == [s['table'] for s in source['sections'] if s.get('table')]
        headings = [s['heading'] for s in source['sections']]
        checks['required_original_headings_in_order'] = headings[:10] == HEADINGS
        checks['native_heading_styles'] = all(any(p.text == h and p.style.name.startswith('Heading') for p in document.paragraphs) for h in headings)
        word_count = len(re.findall(r"\b[\w'-]+\b", ' '.join(v for s in source['sections'] for k in ('paragraphs','bullets') for v in s.get(k, []))))
        checks['minimum_900_words_excluding_tables'] = word_count >= 900
        checks['expected_native_table_count'] = len(tables) == (3 if stage == 1 else 4)
        receipt_path = root / 'project' / f'review-v{stage}' / 'review.json'
        receipt = json.loads(receipt_path.read_text(encoding='utf-8')) if receipt_path.is_file() else {}
        artifact = root / 'project' / f'proposal-v{stage}.docx'
        pages = receipt.get('pages',0)
        checks['render_receipt_matches_artifact'] = receipt.get('artifact_sha256') == hashlib.sha256(artifact.read_bytes()).hexdigest()
        checks['at_least_3_rendered_pages'] = pages >= 3
        checks['all_page_images_present'] = pages > 0 and all((receipt_path.parent / f'page-{i:03}.png').is_file() for i in range(1,pages+1))
        checks['structural_render_review'] = receipt.get('render_status') == 'PASS' and receipt.get('structural_review',{}).get('structural_status') == 'PASS'
        checks['earlier_versions_unchanged'] = runtime['immutable_inputs_preserved']
        budget = next((s.get('table', []) for s in source['sections'] if s['heading'] == 'Budget'), [])
        amount = lambda value: int(re.sub(r'[^0-9]', '', value))
        checks['budget_arithmetic'] = bool(budget) and all(len(r) == 4 for r in budget) and sum(amount(r[3]) for r in budget[1:-1]) == amount(budget[-1][3]) == (375000 if stage == 3 else 360000)
        if stage == 1:
            expected = [['Item','Quantity','Unit cost (ZAR)','Total (ZAR)'], ['Workstations','20','12,000','240,000'], ['Network','1','35,000','35,000'], ['Training','1','60,000','60,000'], ['Contingency','1','25,000','25,000'], ['Total','','','360,000']]
            checks['exact_requested_budget_cells'] = budget == expected
            checks['exact_original_section_count'] = len(headings) == 10
        elif stage == 2:
            before = json.loads((root / 'proposal-source-v1.json').read_text(encoding='utf-8-sig'))
            previous = Document(root / 'project' / 'proposal-v1.docx')
            previous_paragraphs = [p.text for p in previous.paragraphs]
            previous_tables = [[[c.text for c in r.cells] for r in t.rows] for t in previous.tables]
            checks['native_existing_content_preserved'] = paragraphs[:len(previous_paragraphs)] == previous_paragraphs and tables[:len(previous_tables)] == previous_tables
            checks['all_original_sections_unchanged'] = source['sections'][:10] == before['sections']
            checks['all_non_section_fields_unchanged'] = {k:v for k,v in source.items() if k!='sections'} == {k:v for k,v in before.items() if k!='sections'}
            appendix = source['sections'][-1]
            checks['exactly_one_requested_appendix'] = len(headings) == 11 and appendix['heading'] == 'Appendix - Incident Response'
            checks['appendix_at_least_120_words'] = len(' '.join(appendix.get('paragraphs',[]) + appendix.get('bullets',[])).split()) >= 120
            checks['exact_appendix_table'] = appendix.get('table') == [['Priority','Response target'],['P1','30 minutes'],['P2','4 hours']]
        else:
            before = json.loads((root / 'proposal-source-v2.json').read_text(encoding='utf-8-sig'))
            checks['only_exact_requested_changes'] = source == changed(before)
            checks['old_budget_values_removed'] = '60,000' not in actual and '360,000' not in actual
            previous = Document(root / 'project' / 'proposal-v2.docx')
            checks['native_only_requested_changes'] = paragraphs == changed([p.text for p in previous.paragraphs]) and tables == changed([[[c.text for c in r.cells] for r in t.rows] for t in previous.tables])
        component_checks = (['native_existing_content_preserved','all_original_sections_unchanged','all_non_section_fields_unchanged','exactly_one_requested_appendix','appendix_at_least_120_words','exact_appendix_table'] if stage == 2 else ['native_only_requested_changes','only_exact_requested_changes','old_budget_values_removed'] if stage == 3 else [])
        component_checks += ['runtime_completed','all_authored_text_in_docx','all_authored_native_tables_match','earlier_versions_unchanged','render_receipt_matches_artifact','all_page_images_present']
        report = {'passed':all(checks.values()), 'checks':checks, 'words_excluding_tables':word_count,
                  'isolated_revision_component_passed': all(checks[k] for k in component_checks) if stage > 1 else None,
                  'semantic_revision_component_passed': all(checks[k] for k in component_checks if k != 'runtime_completed') if stage > 1 else None,
                  'native_tables':len(tables),'pages':pages,'visual_review':'PENDING', 'errors':errors}
    except Exception as error:
        errors.append(f'{type(error).__name__}: {error}')
        report = {'passed':False,'checks':checks,'errors':errors,'visual_review':'PENDING'}
    save(root / f'stage-{stage}-verification.json', report)
    print(json.dumps(report,indent=2))
    return report['passed']

if __name__ == '__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('action',choices=['setup','verify'])
    parser.add_argument('root',type=Path)
    parser.add_argument('--stage',type=int,choices=[1,2,3],default=1)
    args=parser.parse_args()
    if args.action == 'setup':
        setup(args.root)
    else:
        raise SystemExit(0 if verify(args.root,args.stage) else 1)
