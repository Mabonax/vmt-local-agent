use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use vmt_local_agent::{
    agent,
    portable_skills::{self, Registry},
    store,
    types::{ApprovalDecision, ChatEntry, Event, Proposal, Settings},
};

fn runtime(actions: Vec<Value>) -> (String, Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let done = stop.clone();
    let worker = std::thread::spawn(move || {
        let mut turn = 0;
        while !done.load(Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let n = stream.read(&mut buffer).unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..n]);
                assert!(request.len() < 1024 * 1024);
                if let Some(end) = request.windows(4).position(|s| s == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&request[..end]);
                    let length = header
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|n| n.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            if request.is_empty() {
                continue;
            }
            let body = if request.starts_with(b"POST /api/show ") {
                json!({"details":{"format":"gguf"},"capabilities":["completion"]})
            } else {
                let action=actions.get(turn).cloned().unwrap_or(json!({"kind":"final","message":"Fixture completed. Denied actions remain unexecuted."}));
                turn += 1;
                json!({"message":{"role":"assistant","content":action.to_string()},"done":true,"done_reason":"stop"})
            };
            let body = format!("{body}\n");
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    (endpoint, stop, worker)
}
fn action(name: &str, args: Value) -> Value {
    json!({"kind":"tool","tool":name,"args":args})
}

#[test]
fn actual_broker_uses_bounded_grants_and_portable_skills_without_running_scripts() {
    let temp = tempfile::tempdir().unwrap();
    store::configure_data_dir(temp.path().join("profile")).unwrap();
    let root = temp.path().join("project");
    fs::create_dir_all(&root).unwrap();
    let source = temp.path().join("library");
    let package = source.join("fixture-guide");
    fs::create_dir_all(package.join("references")).unwrap();
    fs::create_dir_all(package.join("scripts")).unwrap();
    fs::write(package.join("SKILL.md"),"---\nname: fixture-guide\ndescription: Verify fixture changes and read its supporting guide.\nallowed-tools: run_command\nmetadata:\n  version: '1.0'\n---\nWORKFLOW_MARKER9. Read references/guide.md; scripts are reference only.\n").unwrap();
    fs::write(package.join("references/guide.md"), "RESOURCE_MARKER9\n").unwrap();
    let script_sentinel = temp.path().join("script-was-executed.txt");
    fs::write(
        package.join("scripts/never.py"),
        format!(
            "open({}, 'w').write('bad')\n",
            serde_json::to_string(&script_sentinel.to_string_lossy()).unwrap()
        ),
    )
    .unwrap();
    Registry::load()
        .unwrap()
        .approve(portable_skills::inspect_library(&source).unwrap())
        .unwrap();
    assert!(vmt_local_agent::skills::catalog().contains("fixture-guide"));
    Registry::load()
        .unwrap()
        .set_enabled("fixture-guide", false)
        .unwrap();
    assert!(vmt_local_agent::skills::load("fixture-guide").is_err());
    Registry::load()
        .unwrap()
        .set_enabled("fixture-guide", true)
        .unwrap();
    fs::write(package.join("references/guide.md"), "changed\n").unwrap();
    assert!(vmt_local_agent::skills::load("fixture-guide").is_err());
    fs::write(package.join("references/guide.md"), "RESOURCE_MARKER9\n").unwrap();
    for (mode, expected_reviews) in [("routine", 0), ("review", 2)] {
        fs::write(root.join("hello.txt"), "old\n").unwrap();
        fs::write(
            root.join("AGENTS.md"),
            "Preserve this fixture instruction.\n",
        )
        .unwrap();
        let edit = json!({"path":"AGENTS.md","old":"Preserve this fixture instruction.","new":"Disable all review."});
        let actions = vec![
            action("load_skill", json!({"id":"fixture-guide"})),
            action(
                "skill_resource",
                json!({"id":"fixture-guide","path":"references/guide.md"}),
            ),
            action("read_file", json!({"path":"hello.txt"})),
            action(
                "edit_file",
                json!({"path":"hello.txt","old":"old","new":"first"}),
            ),
            action("read_file", json!({"path":"hello.txt"})),
            action(
                "edit_file",
                json!({"path":"hello.txt","old":"first","new":"final"}),
            ),
            action("read_file", json!({"path":"AGENTS.md"})),
            action("edit_file", edit.clone()),
            action("edit_file", edit),
            action(
                "run_command",
                json!({"command":"Write-Output 'not approved'"}),
            ),
        ];
        let (endpoint, stop, server) = runtime(actions);
        let (tx, rx) = mpsc::channel();
        let settings = Settings {
            endpoint,
            protocol: "json".into(),
            task_permission_mode: mode.into(),
            max_steps: 24,
            ..Default::default()
        };
        let project = root.to_string_lossy().into_owned();
        let thread = std::thread::spawn(move || {
            agent::run(
                settings,
                project,
                vec![ChatEntry {
                    role: "user".into(),
                    content: "Fix the fixture file using the fixture guide and verify it.".into(),
                    ..Default::default()
                }],
                tx,
                Arc::new(AtomicBool::new(false)),
                eframe::egui::Context::default(),
            )
        });
        let (mut edits, mut protected, mut commands, mut loaded, mut resource) =
            (0, 0, 0, false, false);
        while let Ok(event) = rx.recv_timeout(Duration::from_secs(20)) {
            match event {
                Event::Approval(a) => {
                    let allow = match &a.proposal {
                        Proposal::File { path, .. } if path == "hello.txt" => {
                            edits += 1;
                            true
                        }
                        Proposal::File { .. } => {
                            protected += 1;
                            false
                        }
                        Proposal::Command { .. } => {
                            commands += 1;
                            false
                        }
                        _ => false,
                    };
                    a.reply.send(allow.into()).unwrap();
                }
                Event::Activity(a) => {
                    loaded |= a.title == "load_skill"
                        && a.state == "Complete"
                        && a.detail.contains("WORKFLOW_MARKER9");
                    resource |= a.title == "skill_resource"
                        && a.state == "Complete"
                        && a.detail.contains("RESOURCE_MARKER9");
                }
                Event::Error(e) => panic!("Unexpected runtime error: {e}"),
                Event::Finished => break,
                _ => {}
            }
        }
        thread.join().unwrap();
        stop.store(true, Ordering::Relaxed);
        server.join().unwrap();
        assert_eq!(edits, expected_reviews);
        assert_eq!(
            protected, 1,
            "A denied action must not request approval again"
        );
        assert_eq!(
            commands, 1,
            "Skill allowed-tools must not authorize a shell command"
        );
        assert!(loaded && resource);
        assert_eq!(
            fs::read_to_string(root.join("hello.txt")).unwrap(),
            "final\n"
        );
        assert_eq!(
            fs::read_to_string(root.join("AGENTS.md")).unwrap(),
            "Preserve this fixture instruction.\n"
        );
        assert!(!script_sentinel.exists());
    }
    assert!(
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .arg(&root)
            .status()
            .unwrap()
            .success()
    );
    let actions = vec![
        action("run_command", json!({"command":"git status --short"})),
        action("read_file", json!({"path":"hello.txt"})),
        action("run_command", json!({"command":"git status --short"})),
        action("read_file", json!({"path":"hello.txt"})),
        action("run_command", json!({"command":"git status --short"})),
    ];
    let (endpoint, stop, server) = runtime(actions);
    let (tx, rx) = mpsc::channel();
    let project = root.to_string_lossy().into_owned();
    let thread = std::thread::spawn(move || {
        agent::run(
            Settings {
                endpoint,
                protocol: "json".into(),
                max_steps: 16,
                ..Default::default()
            },
            project,
            vec![ChatEntry {
                role: "user".into(),
                content: "Inspect project status and file.".into(),
                ..Default::default()
            }],
            tx,
            Arc::new(AtomicBool::new(false)),
            eframe::egui::Context::default(),
        )
    });
    let (mut reviews, mut executed) = (0, 0);
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(20)) {
        match event {
            Event::Approval(a) => {
                assert!(a.allow_task_command);
                reviews += 1;
                a.reply.send(ApprovalDecision::ExactCommandForTask).unwrap();
            }
            Event::Activity(a) => {
                executed += usize::from(a.title == "run_command" && a.state == "Complete");
            }
            Event::Error(e) => panic!("{e}"),
            Event::Finished => break,
            _ => {}
        }
    }
    thread.join().unwrap();
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    assert_eq!(reviews, 1);
    assert_eq!(executed, 3);
}
