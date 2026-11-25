"""Trusted built-in workflow templates. Host selects names explicitly."""
import copy

TEMPLATES={
 "pdf_preview": {
  "steps":[
   {"id":"generate","tool":"pdf.generate","args":{"title":{"ref":"input.title"},"body":{"ref":"input.body"}}},
   {"id":"validate","tool":"artifact.inspect","args":{"artifact_id":{"ref":"step.generate.artifact_id"}},"gates":[{"value":{"ref":"step.validate.format"},"op":"eq","target":"pdf"}]},
   {"id":"render","tool":"pdf.render","args":{"artifact_id":{"ref":"step.generate.artifact_id"},"page":1,"dpi":72}},
   {"id":"inspect","tool":"image.inspect","args":{"artifact_id":{"ref":"step.render.artifact_id"}},"gates":[{"value":{"ref":"step.inspect.nonwhite_fraction"},"op":"gt","target":0}]},
   {"id":"hash","tool":"artifact.hash","args":{"artifact_id":{"ref":"step.generate.artifact_id"}},"gates":[{"value":{"ref":"step.hash.sha256"},"op":"eq","target":{"ref":"step.generate.sha256"}}]}
  ],
  "outputs":{"document":{"ref":"step.generate"},"preview":{"ref":"step.render"},"metrics":{"ref":"step.inspect"}}
 }
}
TEMPLATES["word_report"]={
 "steps":[
  {"id":"generate","tool":"office.docx","args":{"title":{"ref":"input.title"},"sections":[{"heading":"Report","body":{"ref":"input.body"}}]}},
  {"id":"validate","tool":"artifact.inspect","args":{"artifact_id":{"ref":"step.generate.artifact_id"}},"gates":[{"value":{"ref":"step.validate.format"},"op":"eq","target":"docx"}]},
  {"id":"hash","tool":"artifact.hash","args":{"artifact_id":{"ref":"step.generate.artifact_id"}},"gates":[{"value":{"ref":"step.hash.sha256"},"op":"eq","target":{"ref":"step.generate.sha256"}}]}
 ],
 "outputs":{"document":{"ref":"step.generate"},"structure":{"ref":"step.validate"}}
}

def required_tools(name):
    return {step["tool"] for step in TEMPLATES[name]["steps"]}

def get_template(name):
    return copy.deepcopy(TEMPLATES[name])
