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
                a.reply.send(allow)?;
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
