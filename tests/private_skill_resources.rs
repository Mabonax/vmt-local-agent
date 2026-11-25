use vmt_local_agent::{
    portable_skills::{Registry, inspect_library},
    store,
    workspace::Workspace,
};
#[test]
fn registered_private_skills_work_without_exposing_profile_files_to_model_tools() {
    let root = tempfile::tempdir().unwrap();
    let profile = root.path().join("profile");
    store::configure_data_dir(profile.clone()).unwrap();
    let library = profile.join("skill-packages");
    let skill = library.join("test-design");
    std::fs::create_dir_all(skill.join("references")).unwrap();
    std::fs::write(skill.join("SKILL.md"),"---\nname: test-design\ndescription: Synthetic registered design guidance\n---\nUse a readable layout.").unwrap();
    std::fs::write(
        skill.join("references/design.md"),
        "Use navy and teal with clear hierarchy.",
    )
    .unwrap();
    Registry::load()
        .unwrap()
        .approve(inspect_library(&library).unwrap())
        .unwrap();
    let registry = Registry::load().unwrap();
    assert!(
        registry
            .load_skill("test-design")
            .unwrap()
            .contains("readable layout")
    );
    assert!(
        registry
            .resource("test-design", "references/design.md")
            .unwrap()
            .contains("navy and teal")
    );
    assert!(Workspace::new(&skill).unwrap().read("SKILL.md").is_err());
    assert!(
        Workspace::new(root.path())
            .unwrap()
            .read("profile/skill-packages/test-design/SKILL.md")
            .is_err()
    );
    assert!(
        registry
            .resource("test-design", "../../settings.json")
            .is_err()
    );
    std::fs::write(skill.join("references/design.md"), "Changed after review").unwrap();
    assert!(registry.load_skill("test-design").is_err());
}
