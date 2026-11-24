//! Explicit disposable acceptance checks. Never runs on application startup.
use crate::{
    agent, execution, knowledge, mcp, memory, research, skills, store,
    task_journal::TaskJournal,
    tools::{Prepared, Tools, diff},
    types::{ChatEntry, Event, Proposal, Settings},
    workspace::Workspace,
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

fn root() -> Result<PathBuf> {
    let path = std::env::current_dir()?.join(".local/verification-04");
    ensure!(
        store::data_dir().starts_with(path.join("app-data")),
        "Run through scripts/verify-0.4.ps1 to isolate acceptance data"
    );
    fs::create_dir_all(&path)?;
    Ok(path)
}
fn tools_at(name: &str) -> Result<Tools> {
    let path = root()?.join(name);
    fs::create_dir_all(&path)?;
    Ok(Tools {
        workspace: Workspace::new(path)?,
        observed: HashMap::new(),
    })
}
fn approved(
    tools: &mut Tools,
    name: &str,
    args: Value,
    journal: &mut TaskJournal,
    web: bool,
) -> Result<String> {
    let Prepared::Review(proposal) = tools.prepare(name, &args, false, web)? else {
        bail!("Expected review for {name}")
    };
    println!("EXPLICIT FIXTURE APPROVAL {name}: {}", diff(&proposal));
    journal.record("approval", name, "approved")?;
    execution::execute_reviewed(
        tools,
        name,
        &proposal,
        false,
        web,
        journal,
        &Arc::new(AtomicBool::new(false)),
    )
}
pub fn runtime() -> Result<()> {
    let mut t = tools_at("runtime")?;
    let other = tools_at("other-project")?;
    let mut journal = TaskJournal::start(&t.workspace)?;
    let content = "Synthetic acceptance decision: use Rust unit tests.";
    let memory_before = memory::search(&t.workspace, content)?;
    let _ = t.prepare(
        "remember_project",
        &json!({"content":content}),
        false,
        false,
    )?;
    ensure!(
        memory::search(&t.workspace, content)? == memory_before,
        "Unapproved memory was saved"
    );
    for (name, args) in [
        ("remember_project", json!({"content":content})),
        ("workflow_start", json!({"kind":"feature"})),
        ("index_knowledge", json!({"model":"embeddinggemma"})),
        ("mcp_tools", json!({"server":"fixture"})),
        ("run_command", json!({"command":"echo blocked"})),
        ("write_file", json!({"path":"blocked.txt","content":"no"})),
    ] {
        ensure!(
            t.prepare(name, &args, true, true).is_err(),
            "Read-only bypass: {name}"
        );
    }
    approved(
        &mut t,
        "remember_project",
        json!({"content":content}),
        &mut journal,
        false,
    )?;
    ensure!(memory::search(&t.workspace, "Synthetic acceptance")?.contains(content));
    ensure!(!memory::search(&other.workspace, "Synthetic acceptance")?.contains(content));
    for id in [
        "rust-development",
        "feature-delivery",
        "word-documents",
        "pdf-documents",
        "spreadsheets",
        "presentations",
        "artcraft-suite",
    ] {
        ensure!(!skills::load(id)?.is_empty());
    }
    approved(
        &mut t,
        "workflow_start",
        json!({"kind":"feature"}),
        &mut journal,
        false,
    )?;
    let _ = t.prepare(
        "workflow_advance",
        &json!({"evidence":"denied fixture"}),
        false,
        false,
    )?;
    ensure!(crate::workflows::status(&t.workspace)?.contains("inspect"));
    for stage in ["plan", "implement", "verify", "report"] {
        approved(
            &mut t,
            "workflow_advance",
            json!({"evidence":"Synthetic runtime fixture stage"}),
            &mut journal,
            false,
        )?;
        ensure!(crate::workflows::status(&t.workspace)?.contains(stage));
    }
    approved(
        &mut t,
        "workflow_finish",
        json!({"evidence":"All runtime fixture transitions independently checked"}),
        &mut journal,
        false,
    )?;
    ensure!(crate::workflows::status(&t.workspace)?.contains("No active"));
    fs::write(t.workspace.root.join("edit.txt"), "before")?;
    t.prepare("read_file", &json!({"path":"edit.txt"}), false, false)?;
    let Prepared::Review(edit) = t.prepare(
        "edit_file",
        &json!({"path":"edit.txt","old":"before","new":"after"}),
        false,
        false,
    )?
    else {
        bail!("Missing review")
    };
    fs::write(t.workspace.root.join("edit.txt"), "concurrent user edit")?;
    ensure!(
        execution::execute_reviewed(
            &mut t,
            "edit_file",
            &edit,
            false,
            false,
            &mut journal,
            &Arc::new(AtomicBool::new(false))
        )
        .is_err()
    );
    ensure!(t.workspace.read("edit.txt")? == "concurrent user edit");
    ensure!(
        approved(
            &mut t,
            "run_command",
            json!({"command":"Write-Output 'expected failure'; exit 7"}),
            &mut journal,
            false
        )
        .is_err()
    );
    let output = approved(
        &mut t,
        "run_command",
        json!({"command":"Write-Output 'VMT_RUNTIME_OK'"}),
        &mut journal,
        false,
    )?;
    ensure!(output.contains("VMT_RUNTIME_OK"));
    let mut interrupted = TaskJournal::start(&t.workspace)?;
    interrupted.record("dispatch", "run_command", "uncertain-until-result")?;
    drop(interrupted);
    ensure!(crate::task_journal::recent_for(&t.workspace, 25)?.contains("uncertain effects"));
    journal.finish("completed")?;
    println!(
        "PASS runtime: policy, approval preparation/denial, concurrent edit protection, command failure/success, skills, memory isolation, workflow transitions and uncertain journal."
    );
    Ok(())
}
pub fn resume() -> Result<()> {
    let t = tools_at("runtime")?;
    ensure!(memory::search(&t.workspace, "Synthetic acceptance")?.contains("use Rust unit tests"));
    ensure!(crate::task_journal::recent_for(&t.workspace, 25)?.contains("uncertain effects"));
    if let Ok(fixture) = fs::read_to_string(root()?.join("latest-scenario.txt")) {
        let scenario = tools_at(fixture.trim())?;
        ensure!(
            memory::search(&scenario.workspace, "Acceptance fixture uses Rust")?.contains("NOTE")
        );
        ensure!(crate::task_journal::recent_for(&scenario.workspace, 25)?.contains("completed"));
        ensure!(scenario.workspace.read("hello.txt")? == "Hello Rust\n");
        println!(
            "PASS restarted scenario: actual file, reviewed note and completed task history remain."
        );
    }
    println!(
        "PASS new process: persisted project note and uncertain task history retrieved without replay."
    );
    Ok(())
}
pub fn scenario(model: &str) -> Result<()> {
    let fixture = format!("scenario-{}", uuid::Uuid::new_v4());
    let t = tools_at(&fixture)?;
    fs::create_dir_all(t.workspace.root.join("src"))?;
    fs::write(
        t.workspace.root.join("Cargo.toml"),
        "[package]\nname=\"vmt-disposable-acceptance\"\nversion=\"0.1.0\"\nedition=\"2024\"\n",
    )?;
    fs::write(
        t.workspace.root.join("README.md"),
        "A disposable Rust library. greeting() loads hello.txt; the test expects Hello Rust. No dependencies.\n",
    )?;
    fs::write(
        t.workspace.root.join("src/lib.rs"),
        "pub fn greeting() -> &'static str { include_str!(\"../hello.txt\").trim() }\n#[test] fn greeting_contract() { assert_eq!(greeting(), \"Hello Rust\"); }\n",
    )?;
    model_check(
        model,
        "agent",
        "native",
        "Inspect this Rust repository and explain its architecture. Load rust-development. Start a feature workflow and advance through its stages with evidence. Read src/lib.rs and hello.txt. First run exactly cargo test --offline to reproduce the failure. Diagnose it, propose a limited edit of hello.txt replacing Hello world with Hello Rust. After approval, reread the file and run exactly cargo test --offline again. Finish the workflow, then request remember_project with the exact note: Acceptance fixture uses Rust and reviewed tools. Finally accurately report the observed change and test results, including the initial failure.",
        &fixture,
    )?;
    ensure!(
        t.workspace.read("hello.txt")? == "Hello Rust\n",
        "Scenario edit missing"
    );
    ensure!(
        memory::search(&t.workspace, "Acceptance fixture uses Rust")?.contains("NOTE"),
        "Scenario memory missing"
    );
    ensure!(
        crate::workflows::status(&t.workspace)?.contains("No active"),
        "Workflow not finished"
    );
    println!("PASS scenario independently verified file, note and finished workflow.");
    fs::write(root()?.join("latest-scenario.txt"), &fixture)?;
    Ok(())
}

/// Synthetic stdio server used only by the explicit acceptance binary.
pub fn mcp_fixture() -> Result<()> {
    let behavior = std::env::args().nth(2).unwrap_or_default();
    let mut count = 0;
    for line in std::io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line?)?;
        let Some(id) = request.get("id") else {
            continue;
        };
        let result = match request["method"].as_str().unwrap_or("") {
            "initialize" => {
                json!({"protocolVersion":if behavior=="unsupported" {"2099-01-01"} else {"2025-06-18"},"capabilities":{"tools":{}},"serverInfo":{"name":"vmt-fixture","version":"1"}})
            }
            "tools/list" => {
                if request["params"]["cursor"].is_null() {
                    json!({"tools":[{"name":"echo","inputSchema":{"type":"object"}}],"nextCursor":"page2"})
                } else {
                    json!({"tools":[{"name":"sleep","inputSchema":{"type":"object"}},{"name":"oversize","inputSchema":{"type":"object"}},{"name":"error","inputSchema":{"type":"object"}}]})
                }
            }
            "tools/call" => {
                match request["params"]["name"].as_str().unwrap_or("") {
                    "sleep" => std::thread::sleep(Duration::from_secs(10)),
                    "oversize" => {
                        println!("{}", "x".repeat(128 * 1024 + 1));
                        std::io::stdout().flush()?;
                        continue;
                    }
                    "error" => {
                        println!(
                            "{}",
                            json!({"jsonrpc":"2.0","id":id,"result":{"isError":true,"content":[{"type":"text","text":"fixture failure"}]}})
                        );
                        std::io::stdout().flush()?;
                        continue;
                    }
                    _ => {}
                }
                count += 1;
                json!({"content":[{"type":"text","text":format!("fixture count {count}; secret inherited {}",std::env::var_os("VMT_SYNTHETIC_SECRET").is_some())}]})
            }
            _ => json!({}),
        };
        println!("{}", json!({"jsonrpc":"2.0","id":id,"result":result}));
        std::io::stdout().flush()?;
    }
    Ok(())
}
pub fn craft() -> Result<()> {
    let mut t = tools_at("craft")?;
    let output = t.workspace.root.join("outputs");
    fs::create_dir_all(&output)?;
    let mut servers = vec![
        json!({"id":"fixture","command":std::env::current_exe()?,"args":["--mcp-fixture"]}),
        json!({"id":"unsupported","command":std::env::current_exe()?,"args":["--mcp-fixture","unsupported"]}),
    ];
    for (id, exe) in [
        (
            "effectcraft",
            r"C:\Program Files\ArtCraft\EffectCraft\effectcraft-cli.exe",
        ),
        (
            "figcraft",
            r"C:\Program Files\ArtCraft\FigCraft\figcraft-cli.exe",
        ),
        (
            "vectorcraft",
            r"C:\Program Files\VectorCraft\vectorcraft-cli.exe",
        ),
    ] {
        if std::path::Path::new(exe).is_file() {
            let mut args = vec!["mcp".to_string()];
            if id == "vectorcraft" {
                args.push("--headless".into());
            }
            args.extend([
                "--automation-read-root".into(),
                t.workspace.root.to_string_lossy().into_owned(),
                "--automation-write-root".into(),
                output.to_string_lossy().into_owned(),
            ]);
            servers.push(json!({"id":id,"command":exe,"args":args}));
        }
    }
    fs::create_dir_all(store::data_dir())?;
    let registry = store::data_dir().join("mcp-servers.json");
    fs::write(&registry, serde_json::to_vec(&json!({"servers":servers}))?)?;
    let mut journal = TaskJournal::start(&t.workspace)?;
    approved(
        &mut t,
        "mcp_tools",
        json!({"server":"fixture"}),
        &mut journal,
        false,
    )?;
    ensure!(
        mcp::tool_schema("fixture", "sleep")?.contains("sleep"),
        "Pagination did not discover second page"
    );
    for count in [1, 2] {
        let result = approved(
            &mut t,
            "mcp_call",
            json!({"server":"fixture","tool":"echo","arguments":{}}),
            &mut journal,
            false,
        )?;
        ensure!(
            result.contains(&format!("fixture count {count}"))
                && result.contains("secret inherited false"),
            "Session/environment isolation failed"
        );
    }
    let Prepared::Review(snapshot) =
        t.prepare("mcp_tools", &json!({"server":"fixture"}), false, false)?
    else {
        bail!("Missing snapshot")
    };
    let mut changed = servers.clone();
    changed[0]["args"] = json!(["--mcp-fixture", "changed"]);
    fs::write(&registry, serde_json::to_vec(&json!({"servers":changed}))?)?;
    ensure!(
        execution::execute_reviewed(
            &mut t,
            "mcp_tools",
            &snapshot,
            false,
            false,
            &mut journal,
            &Arc::new(AtomicBool::new(false))
        )
        .unwrap_err()
        .to_string()
        .contains("configuration changed")
    );
    fs::write(&registry, serde_json::to_vec(&json!({"servers":servers}))?)?;
    ensure!(
        t.prepare(
            "mcp_call",
            &json!({"server":"fixture","tool":"invented","arguments":{}}),
            false,
            false
        )
        .is_err()
    );
    ensure!(
        approved(
            &mut t,
            "mcp_tools",
            json!({"server":"unsupported"}),
            &mut journal,
            false
        )
        .is_err()
    );
    ensure!(
        approved(
            &mut t,
            "mcp_call",
            json!({"server":"fixture","tool":"error","arguments":{}}),
            &mut journal,
            false
        )
        .is_err()
    );
    approved(
        &mut t,
        "mcp_tools",
        json!({"server":"fixture"}),
        &mut journal,
        false,
    )?;
    let Prepared::Review(sleep) = t.prepare(
        "mcp_call",
        &json!({"server":"fixture","tool":"sleep","arguments":{}}),
        false,
        false,
    )?
    else {
        bail!("Missing review")
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        stop.store(true, Ordering::Relaxed)
    });
    let began = Instant::now();
    ensure!(
        execution::execute_reviewed(
            &mut t,
            "mcp_call",
            &sleep,
            false,
            false,
            &mut journal,
            &cancel
        )
        .is_err()
    );
    ensure!(began.elapsed() < Duration::from_secs(5));
    approved(
        &mut t,
        "mcp_tools",
        json!({"server":"fixture"}),
        &mut journal,
        false,
    )?;
    ensure!(
        approved(
            &mut t,
            "mcp_call",
            json!({"server":"fixture","tool":"oversize","arguments":{}}),
            &mut journal,
            false
        )
        .is_err()
    );
    mcp::disconnect_all();
    println!(
        "PASS synthetic MCP: discovery pagination, session state, schema, immutable review, environment filtering, unsupported protocol, unknown tool, tool failure, cancellation, response cap."
    );
    for server in ["effectcraft", "figcraft", "vectorcraft"] {
        let result = approved(
            &mut t,
            "mcp_tools",
            json!({"server":server}),
            &mut journal,
            false,
        );
        let Ok(list) = result else {
            println!("UNVERIFIED {server}: {}", result.unwrap_err());
            continue;
        };
        println!("{list}");
        if server == "effectcraft" {
            println!("{}", mcp::tool_schema(server, "execute_command")?);
            println!(
                "{}",
                approved(
                    &mut t,
                    "mcp_call",
                    json!({"server":server,"tool":"open_project","arguments":{"new":true}}),
                    &mut journal,
                    false
                )?
            );
            println!(
                "{}",
                approved(
                    &mut t,
                    "mcp_call",
                    json!({"server":server,"tool":"execute_command","arguments":{"command":"comp.new","params":{"name":"VMT Acceptance","width":320,"height":240,"frameRate":24,"duration":1}}}),
                    &mut journal,
                    false
                )?
            );
            let file = output.join("acceptance.ecproj");
            approved(
                &mut t,
                "mcp_call",
                json!({"server":server,"tool":"save_project","arguments":{"path":file}}),
                &mut journal,
                false,
            )?;
            ensure!(file.is_file() && fs::metadata(&file)?.len() > 100);
            mcp::disconnect_all();
            approved(
                &mut t,
                "mcp_tools",
                json!({"server":server}),
                &mut journal,
                false,
            )?;
            approved(
                &mut t,
                "mcp_call",
                json!({"server":server,"tool":"open_project","arguments":{"path":file}}),
                &mut journal,
                false,
            )?;
            let result = approved(
                &mut t,
                "mcp_call",
                json!({"server":server,"tool":"get_project","arguments":{}}),
                &mut journal,
                false,
            )?;
            ensure!(result.contains("VMT Acceptance"));
            println!("PASS EffectCraft create/save/new process/reopen/inspect durable ecproj.");
        } else if server == "figcraft" {
            println!("{}", mcp::tool_schema(server, "create_node")?);
            println!(
                "{}",
                approved(
                    &mut t,
                    "mcp_call",
                    json!({"server":server,"tool":"create_node","arguments":{"type":"rectangle","name":"VMT initial","width":320,"height":200,"props":{"fill":"#65CED6"}}}),
                    &mut journal,
                    false
                )?
            );
            approved(
                &mut t,
                "mcp_call",
                json!({"server":server,"tool":"set_properties","arguments":{"props":{"name":"VMT verified"}}}),
                &mut journal,
                false,
            )?;
            let file = output.join("acceptance.figcraft");
            approved(
                &mut t,
                "mcp_call",
                json!({"server":server,"tool":"save_file","arguments":{"path":file}}),
                &mut journal,
                false,
            )?;
            let svg = output.join("acceptance.svg");
            approved(
                &mut t,
                "mcp_call",
                json!({"server":server,"tool":"export","arguments":{"path":svg,"format":"svg"}}),
                &mut journal,
                false,
            )?;
            ensure!(file.is_file() && fs::read_to_string(svg)?.contains("<svg"));
            mcp::disconnect_all();
            approved(
                &mut t,
                "mcp_tools",
                json!({"server":server}),
                &mut journal,
                false,
            )?;
            approved(
                &mut t,
                "mcp_call",
                json!({"server":server,"tool":"open_file","arguments":{"path":file}}),
                &mut journal,
                false,
            )?;
            let result = approved(
                &mut t,
                "mcp_call",
                json!({"server":server,"tool":"inspect_document","arguments":{}}),
                &mut journal,
                false,
            )?;
            ensure!(result.contains("VMT verified"));
            println!(
                "PASS FigCraft create/edit/save/export/new process/reopen/inspect native and SVG files."
            );
        }
        mcp::disconnect_all();
    }
    journal.finish("completed")?;
    Ok(())
}
pub fn rag() -> Result<()> {
    let mut t = tools_at("rag")?;
    fs::write(
        t.workspace.root.join("accounts.rs"),
        "// Password reset emails and user onboarding are handled by AccountService.\nstruct AccountService;\n",
    )?;
    fs::write(
        t.workspace.root.join("plants.txt"),
        "Sunflowers need water and sunlight for photosynthesis.",
    )?;
    let mut journal = TaskJournal::start(&t.workspace)?;
    approved(
        &mut t,
        "index_knowledge",
        json!({"model":"embeddinggemma:latest"}),
        &mut journal,
        false,
    )?;
    let result = knowledge::search(
        &t.workspace,
        "Which service handles password reset and user onboarding?",
    )?;
    println!("{result}");
    ensure!(
        result.find("accounts.rs") < result.find("plants.txt"),
        "Wrong top-ranked source"
    );
    fs::write(
        t.workspace.root.join("accounts.rs"),
        "// AccountService now supports verified phone recovery.\nstruct AccountService;\n",
    )?;
    ensure!(knowledge::search(&t.workspace, "password reset")?.contains("Password reset"));
    approved(
        &mut t,
        "index_knowledge",
        json!({"model":"embeddinggemma:latest"}),
        &mut journal,
        false,
    )?;
    ensure!(knowledge::search(&t.workspace, "phone recovery")?.contains("phone recovery"));
    ensure!(knowledge::search(&tools_at("no-index")?.workspace, "password reset").is_err());
    ensure!(knowledge::index(&t.workspace, "vmt-missing-embedding-fixture:latest").is_err());
    ensure!(knowledge::index(&t.workspace, "minimax-m3:cloud").is_err());
    journal.finish("completed")?;
    println!(
        "PASS RAG: real local embeddings, ranking, stale snapshot, reindex, scope, missing/cloud model denial."
    );
    Ok(())
}
pub fn research_check() -> Result<()> {
    let mut t = tools_at("research")?;
    let mut journal = TaskJournal::start(&t.workspace)?;
    for url in [
        "http://example.com",
        "https://127.0.0.1",
        "https://100.100.100.100",
        "https://user:pass@example.com",
    ] {
        ensure!(
            t.prepare("web_fetch", &json!({"url":url}), false, true)
                .is_err()
        );
    }
    ensure!(
        t.prepare(
            "web_fetch",
            &json!({"url":"https://example.com"}),
            false,
            false
        )
        .is_err()
    );
    ensure!(
        t.prepare(
            "web_search",
            &json!({"query":"password=fixture"}),
            false,
            true
        )
        .is_err()
    );
    let output = approved(
        &mut t,
        "web_fetch",
        json!({"url":"https://example.com/?vmt_public_fixture=1#section"}),
        &mut journal,
        true,
    )?;
    ensure!(output.contains("Example Domain") && output.contains("UNTRUSTED WEB CONTENT"));
    research::record_fetch(
        &t.workspace,
        "https://example.com/?vmt_public_fixture=1#section",
    )?;
    let sources = research::list_sources(&t.workspace)?;
    ensure!(
        !sources.contains("fixture")
            && !sources.contains("section")
            && sources.contains("https://example.com/")
    );
    if std::env::var_os("OLLAMA_API_KEY").is_none() {
        println!("SKIP authenticated search: OLLAMA_API_KEY is unavailable.");
    } else {
        let result = approved(
            &mut t,
            "web_search",
            json!({"query":"Rust programming language official documentation"}),
            &mut journal,
            true,
        )?;
        ensure!(result.contains("UNTRUSTED SEARCH RESULTS"));
        println!("PASS authenticated public search");
    }
    journal.finish("completed")?;
    println!("PASS public HTTPS fetch, private URL/query guards and stripped provenance.");
    Ok(())
}

/// Real-model acceptance in its own profile; no user's project library is changed.
pub fn project_context(model: &str) -> Result<()> {
    let fixture = tempfile::tempdir()?;
    let project_root = fixture.path().join("source");
    fs::create_dir_all(&project_root)?;
    crate::store::configure_data_dir(fixture.path().join("profile"))?;
    fs::write(
        project_root.join("overview.md"),
        "Acceptance codename: LANTERN-73.\nThis is synthetic project context.\n",
    )?;
    let mut library = crate::projects::Library::default();
    library.create("Source acceptance", &project_root.to_string_lossy())?;
    library.projects[0]
        .references
        .push(crate::projects::Reference {
            id: uuid::Uuid::new_v4().to_string(),
            kind: "file".into(),
            title: "Project overview".into(),
            value: "overview.md".into(),
        });
    library.save()?;
    let settings = Settings {
        model: model.into(),
        mode: "chat".into(),
        ..Default::default()
    };
    let (tx, rx) = mpsc::channel();
    agent::run(settings, project_root.to_string_lossy().into_owned(), vec![ChatEntry {
        role: "user".into(), content: "According to the attached Project overview, what is the acceptance codename? Return only the codename.".into(), ..Default::default()
    }], tx, Arc::new(AtomicBool::new(false)), eframe::egui::Context::default());
    let events: Vec<_> = rx.try_iter().collect();
    ensure!(
        !events
            .iter()
            .any(|e| matches!(e, Event::Error(_) | Event::Approval(_) | Event::Activity(_))),
        "Chat source check failed or attempted tools"
    );
    let answer = events
        .iter()
        .find_map(|e| {
            if let Event::Final(text) = e {
                Some(text)
            } else {
                None
            }
        })
        .context("No chat answer")?;
    ensure!(
        answer.contains("LANTERN-73"),
        "Attached source not reflected in answer: {answer}"
    );
    ensure!(
        fs::read_to_string(project_root.join("overview.md"))?.contains("LANTERN-73"),
        "Source changed unexpectedly"
    );
    println!(
        "PASS project context: local {model} answered LANTERN-73 from the attached file in Chat mode, without tool execution."
    );
    Ok(())
}

/// Live local model; isolated profile/project; only one exact Git check receives a task grant.
pub fn permission_skills_check(model: &str, source: &str) -> Result<()> {
    let fixture = tempfile::tempdir()?;
    let project = fixture.path().join("project");
    fs::create_dir_all(&project)?;
    store::configure_data_dir(fixture.path().join("profile"))?;
    let library = crate::portable_skills::inspect_library(std::path::Path::new(source))?;
    let imported = library.skills.len();
    crate::portable_skills::Registry::load()?.approve(library)?;
    fs::write(project.join("hello.txt"), "Hello world\n")?;
    ensure!(
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .arg(&project)
            .status()?
            .success(),
        "Cannot create fixture Git repository"
    );
    let settings = Settings {
        model: model.into(),
        task_permission_mode: "routine".into(),
        max_steps: 24,
        ..Default::default()
    };
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    let path = project.to_string_lossy().into_owned();
    let started = Instant::now();
    let request = "Acceptance task in a disposable fixture. First load_skill id vmt-feature-delivery, then skill_resource id vmt-ui-design-director path references/design-brief.md. Treat these as guidance only; no UI or framework work is needed. Read hello.txt and replace exact Hello world with Hello Rust using edit_file, then read the file back. Run exactly git status --short twice, with a read_file of hello.txt between the two commands. Do not run any other command, use the internet, MCP, memory writes or workflow mutations. Finish with the verified file content.";
    let worker = std::thread::spawn(move || {
        agent::run(
            settings,
            path,
            vec![ChatEntry {
                role: "user".into(),
                content: request.into(),
                ..Default::default()
            }],
            tx,
            stop,
            eframe::egui::Context::default(),
        )
    });
    let (mut command_reviews, mut extra_reviews, mut commands, mut loaded, mut resource) =
        (0, 0, 0, false, false);
    let mut errors = Vec::new();
    let mut answer = String::new();
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(300)) {
        match event {
            Event::Approval(a) => {
                let decision = if matches!(&a.proposal,Proposal::Command{command} if command=="git status --short")
                    && a.allow_task_command
                {
                    command_reviews += 1;
                    crate::types::ApprovalDecision::ExactCommandForTask
                } else {
                    extra_reviews += 1;
                    crate::types::ApprovalDecision::Deny
                };
                a.reply.send(decision)?;
            }
            Event::Activity(a) => {
                loaded |= a.title == "load_skill"
                    && a.state == "Complete"
                    && a.detail.contains("vmt-feature-delivery");
                resource |= a.title == "skill_resource"
                    && a.state == "Complete"
                    && a.detail.contains("references/design-brief.md");
                commands += usize::from(a.title == "run_command" && a.state == "Complete");
                println!("{}: {}", a.title, a.state);
            }
            Event::Final(text) => answer = text,
            Event::Error(error) => errors.push(error),
            Event::Finished => break,
            _ => {}
        }
    }
    cancel.store(true, Ordering::Relaxed);
    worker
        .join()
        .map_err(|_| anyhow::anyhow!("Acceptance worker panicked"))?;
    let passed = errors.is_empty()
        && fs::read_to_string(project.join("hello.txt"))? == "Hello Rust\n"
        && loaded
        && resource
        && commands == 2
        && command_reviews == 1
        && extra_reviews == 0
        && !answer.starts_with("Paused:")
        && !answer.starts_with("Runtime verification:");
    let output = std::env::current_dir()?.join(".local/verification-06");
    fs::create_dir_all(&output)?;
    let report = serde_json::json!({"passed":passed,"model":model,"portable_skills_registered":imported,"skill_loaded":loaded,"resource_loaded":resource,"commands_completed":commands,"command_reviews":command_reviews,"other_reviews":extra_reviews,"file_content_verified":fs::read_to_string(project.join("hello.txt"))?=="Hello Rust\n","errors":errors,"elapsed_seconds":started.elapsed().as_secs_f64(),"scope":"synthetic fixture and profile; no user project modified"});
    fs::write(
        output.join("permissions-skills-live.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}");
    ensure!(
        passed,
        "Live permission/skills acceptance did not meet all criteria; inspect .local/verification-06/permissions-skills-live.json"
    );
    Ok(())
}

/// Real local model with only exact fixture design actions accepted by the harness.
pub fn design_skills_check(model: &str, source: &str) -> Result<()> {
    let runtime_file = store::data_dir().join("design-runtime.json");
    let runtime = fs::read(runtime_file)?;
    let root = std::env::current_dir()?
        .join(".local")
        .join(format!("design-live-{}", uuid::Uuid::new_v4()));
    let project = root.join("project");
    fs::create_dir_all(&project)?;
    store::configure_data_dir(root.join("profile"))?;
    fs::write(store::data_dir().join("design-runtime.json"), runtime)?;
    let library = crate::portable_skills::inspect_library(std::path::Path::new(source))?;
    ensure!(
        library.skills.len() == 2,
        "Expected the two design packages"
    );
    crate::portable_skills::Registry::load()?.approve(library)?;
    fs::write(
        project.join("content.json"),
        serde_json::to_vec(
            &json!({"title":"Local skill acceptance","subtitle":"A synthetic fixture","sections":[{"heading":"Verified scope","paragraphs":["Create editable files using local tools."],"bullets":["Preserve source text","Render every page"]}]}),
        )?,
    )?;
    // Small RGB PNG fixture, generated without needing an image crate.
    let png = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 2, 8, 2,
        0, 0, 0, 253, 212, 154, 115, 0, 0, 0, 16, 73, 68, 65, 84, 120, 156, 99, 252, 207, 0, 2, 76,
        96, 146, 1, 0, 13, 29, 1, 3, 130, 201, 113, 255, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96,
        130,
    ];
    fs::write(project.join("colour.png"), png)?;
    let settings = Settings {
        model: model.into(),
        max_steps: 32,
        ..Default::default()
    };
    let request = r##"Acceptance in this disposable project. Use tools, then report honest results. First load_skill colour-intelligence and read its references/colour-theory.md with skill_resource. Use design_tool operation palette seed #087F83 mode analogous theme light; then contrast foreground #000000 background #FFFFFF; then extract input colour.png count 2. Load_skill professional-document-design and read its references/input-contract.md. Read content.json. Use design_tool compose input content.json output acceptance.docx, then render input acceptance.docx output review-docx. Then compose input content.json output acceptance.pptx, then render input acceptance.pptx output review-pptx. Do not use commands, internet, MCP, memory writes or alter source files. Each design tool needs separate fixture approval. Do not claim to have viewed PNGs: no image-viewing tool is supplied. State that visual review remains pending. Finish after these operations."##;
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    let path = project.to_string_lossy().into_owned();
    let started = Instant::now();
    let worker = std::thread::spawn(move || {
        agent::run(
            settings,
            path,
            vec![ChatEntry {
                role: "user".into(),
                content: request.into(),
                ..Default::default()
            }],
            tx,
            stop,
            eframe::egui::Context::default(),
        )
    });
    let mut approved = 0;
    let mut denied = 0;
    let mut completed = 0;
    let mut loaded = std::collections::BTreeSet::new();
    let mut references = 0;
    let mut errors = Vec::new();
    let mut answer = String::new();
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(300)) {
        match event {
            Event::Approval(action) => {
                let allowed = if let Proposal::Design { plan } = &action.proposal {
                    let r = &plan.request;
                    match r.operation.as_str() {
                        "palette" => {
                            r.seed.as_deref() == Some("#087F83")
                                && r.mode.as_deref().unwrap_or("analogous") == "analogous"
                                && r.theme.as_deref().unwrap_or("light") == "light"
                        }
                        "contrast" => {
                            r.foreground.as_deref() == Some("#000000")
                                && r.background.as_deref() == Some("#FFFFFF")
                        }
                        "extract" => {
                            r.input.as_deref() == Some("colour.png") && r.count.unwrap_or(6) == 2
                        }
                        "compose" => {
                            r.input.as_deref() == Some("content.json")
                                && matches!(
                                    r.output.as_deref(),
                                    Some("acceptance.docx" | "acceptance.pptx")
                                )
                        }
                        "render" => matches!(
                            (r.input.as_deref(), r.output.as_deref()),
                            (Some("acceptance.docx"), Some("review-docx"))
                                | (Some("acceptance.pptx"), Some("review-pptx"))
                        ),
                        _ => false,
                    }
                } else {
                    false
                };
                if allowed {
                    approved += 1;
                } else {
                    denied += 1;
                }
                action
                    .reply
                    .send(crate::types::ApprovalDecision::from(allowed))?;
            }
            Event::Activity(action) => {
                if action.state == "Complete" {
                    if action.title == "load_skill" {
                        for id in ["colour-intelligence", "professional-document-design"] {
                            if action.detail.contains(id) {
                                loaded.insert(id.to_string());
                            }
                        }
                    }
                    if action.title == "skill_resource" {
                        references += 1;
                    }
                    if crate::design_tools::is_tool(&action.title) {
                        completed += 1;
                    }
                }
                println!("{}: {}", action.title, action.state);
            }
            Event::Final(text) => answer = text,
            Event::Error(error) => errors.push(error),
            Event::Finished => break,
            _ => {}
        }
    }
    cancel.store(true, Ordering::Relaxed);
    worker
        .join()
        .map_err(|_| anyhow::anyhow!("Design acceptance worker panicked"))?;
    let mut artifacts = true;
    for (suffix, pages) in [("docx", 1), ("pptx", 2)] {
        artifacts &= project.join(format!("acceptance.{suffix}")).is_file();
        let review = project.join(format!("review-{suffix}/review.json"));
        artifacts &= fs::read(review)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .is_some_and(|value| value["pages"] == pages && value["visual_review"] == "PENDING");
    }
    let honest_review =
        answer.to_ascii_lowercase().contains("pending") && !answer.starts_with("Paused:");
    let passed = errors.is_empty()
        && denied == 0
        && approved == 7
        && completed == 7
        && loaded.len() == 2
        && references >= 2
        && artifacts
        && honest_review;
    let report = json!({"passed":passed,"model":model,"skills_loaded":loaded,"resources_loaded":references,"design_actions_completed":completed,"explicit_action_reviews":approved,"unrelated_reviews_denied":denied,"artifacts_verified":artifacts,"visual_review_handoff":honest_review,"visual_review":"PENDING","errors":errors,"elapsed_seconds":started.elapsed().as_secs_f64()});
    fs::write(
        root.join("live-report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}\nReport directory: {}", root.display());
    ensure!(
        passed,
        "Live design acceptance incomplete; inspect report and artifacts"
    );
    Ok(())
}

/// User-document evaluation through the actual local model and reviewed broker.
/// Only an unchanged source copy is exposed; no command or arbitrary write is approved.
pub fn document_task_check(model: &str, input: &str) -> Result<()> {
    use sha2::{Digest, Sha256};
    let original = std::path::Path::new(input).canonicalize()?;
    ensure!(
        original.is_file()
            && original
                .extension()
                .is_some_and(|value| value.eq_ignore_ascii_case("docx")),
        "Provide an existing DOCX file"
    );
    ensure!(
        fs::metadata(&original)?.len() <= 16 * 1024 * 1024,
        "Source exceeds DOCX size limit"
    );
    let bytes = fs::read(&original)?;
    let original_hash = format!("{:x}", Sha256::digest(&bytes));
    let registry = fs::read(store::data_dir().join("skill-libraries.json"))?;
    let runtime = fs::read(store::data_dir().join("design-runtime.json"))?;
    let root = std::env::current_dir()?
        .join(".local")
        .join(format!("document-task-{}", uuid::Uuid::new_v4()));
    let project = root.join("project");
    fs::create_dir_all(&project)?;
    fs::write(project.join("CV_Template.docx"), bytes)?;
    store::configure_data_dir(root.join("profile"))?;
    fs::write(store::data_dir().join("skill-libraries.json"), registry)?;
    fs::write(store::data_dir().join("design-runtime.json"), runtime)?;
    let request = "Improve the formatting of CV_Template.docx using BOTH added skills, colour-intelligence and professional-document-design. Preserve every word and placeholder; do not invent or remove CV content. Use the colour skill to choose and audit a restrained accessible accent and apply that accent to the existing document. Create CV_Template_Refined.docx as a real, changed Word file. Use the dedicated existing-document operations, not copying or JSON recomposition. Render the original into review-before and the result into review-cv so a human can compare them. Do not use shell commands, internet, MCP or memory writes. The source must stay unchanged. You have no image-viewing tool: report honestly what was executed and what visual review remains. Work through the actual tools; a response alone is not completion.";
    let settings = Settings {
        model: model.into(),
        context_tokens: 12288,
        max_steps: 40,
        ..Default::default()
    };
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    let path = project.to_string_lossy().into_owned();
    let started = Instant::now();
    let worker = std::thread::spawn(move || {
        agent::run(
            settings,
            path,
            vec![ChatEntry {
                role: "user".into(),
                content: request.into(),
                ..Default::default()
            }],
            tx,
            stop,
            eframe::egui::Context::default(),
        )
    });
    let mut loaded = std::collections::BTreeSet::new();
    let mut operations = Vec::new();
    let mut approvals = 0;
    let mut denied = 0;
    let mut failed_tools = Vec::new();
    let mut answer = String::new();
    let mut errors = Vec::new();
    let mut selected_accent = String::new();
    let mut denied_design_requests = Vec::new();
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(300)) {
        match event {
            Event::Approval(action) => {
                let allow = if let Proposal::Design { plan } = &action.proposal {
                    let r = &plan.request;
                    match r.operation.as_str() {
                        "palette" | "contrast" => true,
                        "inspect_document" => {
                            r.input.as_deref() == Some("CV_Template.docx")
                                || r.input.as_deref() == Some("CV_Template_Refined.docx")
                        }
                        "format_document" => {
                            r.input.as_deref() == Some("CV_Template.docx")
                                && r.output.as_deref() == Some("CV_Template_Refined.docx")
                        }
                        "render" => {
                            r.input.as_deref().is_some_and(|input| {
                                let workspace = Workspace {
                                    root: plan.workspace.clone(),
                                };
                                workspace.resolve(input, false).is_ok_and(|path| {
                                    path == plan.workspace.join("CV_Template.docx")
                                        || path == plan.workspace.join("CV_Template_Refined.docx")
                                })
                            }) && r.output.is_some()
                        }
                        _ => false,
                    }
                } else {
                    false
                };
                if allow {
                    approvals += 1;
                } else {
                    denied += 1;
                    if let Proposal::Design { plan } = &action.proposal {
                        denied_design_requests.push(serde_json::to_value(&plan.request)?);
                    }
                }
                action
                    .reply
                    .send(crate::types::ApprovalDecision::from(allow))?;
            }
            Event::Activity(action) => {
                if action.state == "Complete" {
                    if action.title == "load_skill" {
                        for id in ["colour-intelligence", "professional-document-design"] {
                            if action.detail.contains(id) {
                                loaded.insert(id.to_string());
                            }
                        }
                    }
                    if crate::design_tools::is_tool(&action.title)
                        && let Ok(result) = serde_json::from_str::<Value>(&action.detail)
                    {
                        if result["operation"] == "format_document" {
                            selected_accent = result["accent"].as_str().unwrap_or("").into();
                        }
                        operations.push(result);
                    }
                }
                if action.state == "Failed" {
                    failed_tools.push(json!({"tool":action.title,"error":action.detail}));
                }
                println!("{}: {}", action.title, action.state);
            }
            Event::Final(text) => answer = text,
            Event::Error(error) => errors.push(error),
            Event::Finished => break,
            _ => {}
        }
    }
    cancel.store(true, Ordering::Relaxed);
    worker
        .join()
        .map_err(|_| anyhow::anyhow!("Document task worker panicked"))?;
    let original_preserved = format!("{:x}", Sha256::digest(fs::read(&original)?)) == original_hash
        && format!(
            "{:x}",
            Sha256::digest(fs::read(project.join("CV_Template.docx"))?)
        ) == original_hash;
    let output = project.join("CV_Template_Refined.docx");
    let output_hash = if output.is_file() {
        format!("{:x}", Sha256::digest(fs::read(&output)?))
    } else {
        String::new()
    };
    let palette = operations.iter().any(|value| value.get("roles").is_some());
    let palette_applied = operations
        .iter()
        .filter_map(|value| value["roles"].as_object())
        .any(|roles| {
            roles.values().any(|value| {
                value
                    .as_str()
                    .is_some_and(|colour| colour.eq_ignore_ascii_case(&selected_accent))
            })
        });
    let contrast = operations.iter().any(|value| {
        value["AA_normal"] == true
            && ((value["foreground"]
                .as_str()
                .is_some_and(|colour| colour.eq_ignore_ascii_case(&selected_accent))
                && value["background"] == "#FFFFFF")
                || (value["background"]
                    .as_str()
                    .is_some_and(|colour| colour.eq_ignore_ascii_case(&selected_accent))
                    && value["foreground"] == "#FFFFFF"))
    });
    let content = operations
        .iter()
        .any(|value| value["operation"] == "format_document" && value["content_preserved"] == true);
    let render = project.join("review-cv/review.json").is_file();
    let passed = loaded.len() == 2
        && palette
        && palette_applied
        && contrast
        && content
        && original_preserved
        && !output_hash.is_empty()
        && output_hash != original_hash
        && render
        && errors.is_empty()
        && denied == 0
        && !answer.starts_with("Task incomplete")
        && !answer.starts_with("Paused:");
    let report = json!({"passed":passed,"model":model,"skills_loaded":loaded,"palette_executed":palette,"palette_accent_applied":palette_applied,"contrast_executed":contrast,"formatting_content_preserved":content,"original_preserved":original_preserved,"source_sha256":original_hash,"output_sha256":output_hash,"rendered":render,"accent":selected_accent,"explicit_approvals":approvals,"unexpected_actions_denied":denied,"denied_design_requests":denied_design_requests,"failed_tools":failed_tools,"errors":errors,"elapsed_seconds":started.elapsed().as_secs_f64(),"visual_review":"PENDING"});
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    fs::write(root.join("agent-response.txt"), answer)?;
    println!("{report}\nDocument task directory: {}", root.display());
    ensure!(
        passed,
        "Local document task did not meet its acceptance criteria; inspect the private report"
    );
    Ok(())
}

/// Explicit synthetic operator fixture. Only exact predeclared design requests
/// receive approval; model text cannot expand the allowlist or grant permissions.
pub fn creative_live_check(model: &str, project: &str, protocol: &str) -> Result<()> {
    ensure!(matches!(protocol, "native" | "json"), "Use native or json");
    let workspace = Workspace::new(project)?;
    let suffix = format!("live-{protocol}");
    let allowed = [
        json!({"operation":"palette","seed":"#087F83","mode":"analogous","theme":"light"}),
        json!({"operation":"compose_v2","input":"business-proposal.docx.json","output":format!("{suffix}.docx")}),
        json!({"operation":"compose_v2","input":"business-presentation.pptx.json","output":format!("{suffix}.pptx")}),
        json!({"operation":"svg_compose","input":"event-poster.json","output":format!("{suffix}-poster.png"),"assets":["original-landscape.jpg"]}),
        json!({"operation":"svg_compose","input":"business-card.json","output":format!("{suffix}-card.svg"),"assets":["original-landscape.jpg"]}),
        json!({"operation":"scene_repair_plan","input":"feedback-request.json"}),
    ];
    let request = format!(
        "Work only in this synthetic acceptance project. Create a professional business proposal in Word from business-proposal.docx.json and a presentation from business-presentation.pptx.json, using compose_v2. Create an accessible corporate palette from #087F83, analogous, light. Create an event poster from event-poster.json with approved original-landscape.jpg and a business card from business-card.json using the same image. Propose contrast/layout corrections from feedback-request.json using scene_repair_plan. Load each matching skill once before using it. Do not reload unchanged skills or reread unchanged files; move to execution after inspection. Output names MUST be {suffix}.docx, {suffix}.pptx, {suffix}-poster.png, {suffix}-card.svg. Preserve every source. Use only read tools, skill tools and the six design operations described. No shell, internet, memory writes, workflows, other file changes or vision inference in this reasoning run. Report the provenance supplied by the actual broker and any failures. Visual acceptance remains pending until separately inspected."
    );
    let settings = Settings {
        model: model.into(),
        protocol: protocol.into(),
        context_tokens: 16384,
        max_steps: 40,
        task_minutes: 8,
        web_enabled: false,
        ..Default::default()
    };
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    let path = workspace.root.to_string_lossy().into_owned();
    let started = Instant::now();
    let worker = std::thread::spawn(move || {
        agent::run(
            settings,
            path,
            vec![ChatEntry {
                role: "user".into(),
                content: request,
                ..Default::default()
            }],
            tx,
            stop,
            eframe::egui::Context::default(),
        )
    });
    let (mut approvals, mut denied) = (0, 0);
    let mut completed = Vec::new();
    let mut failures = Vec::new();
    let mut errors = Vec::new();
    let mut answer = String::new();
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(180)) {
        match event {
            Event::Approval(action) => {
                let allow = if let Proposal::Design { plan } = &action.proposal {
                    let mut request = serde_json::to_value(&plan.request)?;
                    request.as_object_mut().unwrap().retain(|_, v| !v.is_null());
                    allowed.contains(&request)
                } else {
                    false
                };
                if allow {
                    approvals += 1;
                } else {
                    denied += 1;
                }
                action.reply.send(allow.into())?;
            }
            Event::Activity(action) => {
                if action.state == "Complete" && crate::design_tools::is_tool(&action.title) {
                    completed.push(json!({"tool":action.title,"detail":action.detail}));
                } else if action.state == "Failed" {
                    failures.push(json!({"tool":action.title,"detail":action.detail}));
                }
                println!("{}: {}", action.title, action.state);
            }
            Event::Final(text) => answer = text,
            Event::Error(error) => errors.push(error),
            Event::Finished => break,
            _ => {}
        }
    }
    cancel.store(true, Ordering::Relaxed);
    worker
        .join()
        .map_err(|_| anyhow::anyhow!("Creative live worker panicked"))?;
    let artifacts = [
        format!("{suffix}.docx"),
        format!("{suffix}.pptx"),
        format!("{suffix}-poster.png"),
        format!("{suffix}-card.svg"),
    ]
    .iter()
    .all(|p| workspace.root.join(p).is_file());
    let passed = errors.is_empty()
        && failures.is_empty()
        && denied == 0
        && completed.len() == 6
        && artifacts
        && !answer.is_empty();
    let report = json!({"passed":passed,"model":model,"protocol":protocol,"approvals":approvals,"denied":denied,"completed":completed,"failed_tools":failures,"errors":errors,"answer":answer,"artifacts_present":artifacts,"seconds":started.elapsed().as_secs_f64()});
    fs::write(
        workspace.root.join(format!("{suffix}-report.json")),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("Live creative {protocol}: passed={passed}, approvals={approvals}, denied={denied}");
    ensure!(
        passed,
        "Live creative run incomplete; inspect retained report"
    );
    Ok(())
}

pub fn model_check(
    model: &str,
    mode: &str,
    protocol: &str,
    request: &str,
    fixture: &str,
) -> Result<()> {
    let t = tools_at(fixture)?;
    if mode == "agent" {
        fs::write(t.workspace.root.join("hello.txt"), "Hello world\n")?;
    }
    let settings = Settings {
        model: model.into(),
        mode: mode.into(),
        protocol: protocol.into(),
        max_steps: 30,
        output_tokens: 4096,
        ..Default::default()
    };
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    let path = t.workspace.root.to_string_lossy().into_owned();
    let history = vec![ChatEntry {
        role: "user".into(),
        content: request.into(),
        ..Default::default()
    }];
    let started = Instant::now();
    let handle = std::thread::spawn(move || {
        agent::run(
            settings,
            path,
            history,
            tx,
            stop,
            eframe::egui::Context::default(),
        )
    });
    let mut errors = Vec::new();
    let mut final_text = String::new();
    let (mut failed_test, mut passed_test, mut loaded_skill) = (false, false, false);
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(300)) {
        match event {
            Event::Approval(a) => {
                let allow = match &a.proposal {
                    Proposal::File {
                        path,
                        before,
                        after,
                    } => {
                        path == "hello.txt"
                            && before.as_deref() == Some("Hello world\n")
                            && after == "Hello Rust\n"
                    }
                    Proposal::MemoryNote { content } => {
                        content == "Acceptance fixture uses Rust and reviewed tools."
                    }
                    Proposal::Workflow { .. } => fixture.starts_with("scenario-"),
                    Proposal::Command { command } => {
                        fixture.starts_with("scenario-") && command == "cargo test --offline"
                    }
                    _ => false,
                };
                println!("review {} {}: {}", a.title, allow, a.detail);
                a.reply.send(allow.into())?;
            }
            Event::Activity(a) => {
                failed_test |= a.title == "run_command"
                    && a.state == "Failed"
                    && a.detail.contains("greeting_contract");
                passed_test |= a.title == "run_command"
                    && a.state == "Complete"
                    && a.detail.contains("1 passed; 0 failed");
                loaded_skill |= a.title == "load_skill"
                    && a.state == "Complete"
                    && a.detail.contains("rust-development");
                println!(
                    "{} {}: {}",
                    a.title,
                    a.state,
                    crate::workspace::clip(&a.detail, 2000)
                );
            }
            Event::Final(text) => {
                println!("FINAL: {text}");
                final_text = text;
            }
            Event::Error(error) => {
                println!("MODEL FAILURE: {error}");
                errors.push(error);
            }
            Event::Finished => break,
            _ => {}
        }
    }
    cancel.store(true, Ordering::Relaxed);
    handle
        .join()
        .map_err(|_| anyhow::anyhow!("Agent worker panicked"))?;
    ensure!(errors.is_empty(), "Model failed: {}", errors.join("; "));
    ensure!(!final_text.is_empty(), "No completed answer");
    ensure!(
        !final_text.starts_with("Runtime verification: turn incomplete"),
        "Runtime reports incomplete work"
    );
    if fixture.starts_with("scenario-") {
        ensure!(
            failed_test && passed_test && loaded_skill,
            "Missing real test failure/recovery or skill evidence"
        );
    }
    if mode == "chat" {
        ensure!(final_text.contains("391"), "Incorrect arithmetic answer");
    }
    println!(
        "Completed model {model}, mode {mode}, protocol {protocol}, elapsed {:.1}s",
        started.elapsed().as_secs_f32()
    );
    Ok(())
}
