use serde_json::json;
use std::{
    fs,
    sync::{Arc, atomic::AtomicBool},
};
use vmt_local_agent::{
    design_tools, execution,
    portable_skills::{self, Registry},
    store,
    task_journal::TaskJournal,
    task_permissions::TaskPermissions,
    tools::{Prepared, Tools},
    types::Proposal,
    workspace::Workspace,
};

#[test]
fn design_actions_cannot_bypass_review_or_change_after_review() {
    let fixture = tempfile::tempdir().unwrap();
    store::configure_data_dir(fixture.path().join("profile")).unwrap();
    let source = fixture.path().join("library/professional-document-design");
    fs::create_dir_all(source.join("scripts")).unwrap();
    fs::write(source.join("SKILL.md"),"---\nname: professional-document-design\ndescription: Fixture only\n---\nDo not execute without review\n").unwrap();
    fs::write(
        source.join("scripts/compose.py"),
        "# Fixture never launched\n",
    )
    .unwrap();
    Registry::load()
        .unwrap()
        .approve(portable_skills::inspect_library(source.parent().unwrap()).unwrap())
        .unwrap();
    let fake_executable = fixture.path().join("fake-runtime.exe");
    fs::write(&fake_executable, "not executed").unwrap();
    design_tools::configure(
        fake_executable.to_str().unwrap(),
        fake_executable.to_str().unwrap(),
    )
    .unwrap();
    let project = fixture.path().join("project");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("input.json"), "{}").unwrap();
    let mut tools = Tools {
        workspace: Workspace::new(&project).unwrap(),
        observed: Default::default(),
    };
    let args = json!({"operation":"compose","input":"input.json","output":"output.docx"});
    assert!(tools.prepare("design_tool", &args, true, false).is_err());
    let Prepared::Review(proposal) = tools.prepare("design_tool", &args, false, false).unwrap()
    else {
        panic!("Design execution must require review")
    };
    let mut grant = TaskPermissions::new(&tools.workspace, true);
    assert!(
        grant
            .consume(&tools.workspace, "design_tool", &proposal, false)
            .is_none()
    );
    assert!(!vmt_local_agent::task_permissions::repeatable_command(
        &proposal
    ));
    let mut journal = TaskJournal::start(&tools.workspace).unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    assert!(
        execution::execute_reviewed(
            &mut tools,
            "run_command",
            &proposal,
            false,
            false,
            &mut journal,
            &cancel
        )
        .unwrap_err()
        .to_string()
        .contains("mismatch")
    );
    fs::write(project.join("input.json"), "{\"changed\":true}").unwrap();
    assert!(
        execution::execute_reviewed(
            &mut tools,
            "design_tool",
            &proposal,
            false,
            false,
            &mut journal,
            &cancel
        )
        .unwrap_err()
        .to_string()
        .contains("Input changed")
    );
    let Prepared::Review(updated) = tools.prepare("design_tool", &args, false, false).unwrap()
    else {
        panic!()
    };
    fs::write(source.join("scripts/compose.py"), "# Unreviewed update\n").unwrap();
    assert!(
        execution::execute_reviewed(
            &mut tools,
            "design_tool",
            &updated,
            false,
            false,
            &mut journal,
            &cancel
        )
        .is_err()
    );
    assert!(!project.join("output.docx").exists());
    if let Proposal::Design { plan } = updated {
        assert!(
            design_tools::execute(&tools.workspace, &plan, &Arc::new(AtomicBool::new(true)))
                .is_err()
        );
    } else {
        panic!()
    }
}
