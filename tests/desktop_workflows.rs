use serde_json::{Value, json};
use std::{
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
    projects::{Library, Reference},
    store,
    types::{ChatEntry, Event, Settings},
    workspace::Workspace,
};

fn mock_runtime(repeated: bool) -> (String, Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let done = stop.clone();
    let handle = std::thread::spawn(move || {
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
            let mut bytes = Vec::new();
            let mut buf = [0; 4096];
            loop {
                let size = stream.read(&mut buf).unwrap();
                if size == 0 {
                    break;
                }
                bytes.extend_from_slice(&buf[..size]);
                if let Some(end) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&bytes[..end]);
                    let len = header
                        .lines()
                        .find_map(|line| {
                            line.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            let header = String::from_utf8_lossy(&bytes);
            let body = if header.starts_with("POST /api/show ") {
                json!({"details":{"format":"gguf"},"capabilities":["completion"]})
            } else {
                let action = if turn < 14 || repeated {
                    json!({"kind":"tool","tool":"read_file","args":{"path":format!("f{}.txt",if repeated{0}else{turn})}})
                } else {
                    json!({"kind":"final","message":"Inspected all 14 fixture files."})
                };
                turn += 1;
                json!({"message":{"role":"assistant","content":action.to_string()},"done":true,"done_reason":"stop"})
            };
            let body = format!("{body}\n");
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        }
    });
    (endpoint, stop, handle)
}

#[test]
fn extended_tasks_pause_and_continue_without_replaying_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    store::configure_data_dir(temp.path().join("profile")).unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir_all(&root).unwrap();
    for n in 0..14 {
        std::fs::write(root.join(format!("f{n}.txt")), format!("fixture {n}\n")).unwrap();
    }
    let mut library = Library::default();
    library.create("Fixture", &root.to_string_lossy()).unwrap();
    library.projects[0].references.push(Reference {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "file".into(),
        title: "Forbidden outside source".into(),
        value: "../private.txt".into(),
    });
    std::fs::write(temp.path().join("private.txt"), "outside-canary").unwrap();
    library.save().unwrap();
    let context = vmt_local_agent::projects::context(&Workspace::new(&root).unwrap()).unwrap();
    assert!(!context.contains("outside-canary"));
    assert!(context.contains("unavailable"));
    for (steps, repeated, expected_reads) in [(24, false, 14), (3, false, 3), (48, true, 2)] {
        let (endpoint, stop, server) = mock_runtime(repeated);
        let settings = Settings {
            endpoint,
            protocol: "json".into(),
            max_steps: steps,
            ..Default::default()
        };
        let (tx, rx) = mpsc::channel();
        agent::run(
            settings,
            root.to_string_lossy().into(),
            vec![ChatEntry {
                role: "user".into(),
                content: "Inspect the fixture project files in order.".into(),
                ..Default::default()
            }],
            tx,
            Arc::new(AtomicBool::new(false)),
            eframe::egui::Context::default(),
        );
        stop.store(true, Ordering::Relaxed);
        server.join().unwrap();
        let events: Vec<_> = rx.try_iter().collect();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::Approval(_) | Event::Error(_))),
            "Unexpected approval or error"
        );
        assert_eq!(events.iter().filter(|e|matches!(e,Event::Activity(a) if a.title=="read_file" && a.state=="Complete")).count(),expected_reads);
        if steps == 24 {
            assert!(
                events.iter().any(
                    |e| matches!(e,Event::Final(s) if s.contains("14 completed tool actions"))
                )
            );
        } else {
            let checkpoint = events
                .iter()
                .find_map(|e| {
                    if let Event::Paused(c) = e {
                        Some(c)
                    } else {
                        None
                    }
                })
                .expect("Checkpoint required");
            assert!(checkpoint.continuation().contains("Do not replay"));
            let serialized = serde_json::to_string(checkpoint).unwrap();
            let reloaded: vmt_local_agent::types::TaskCheckpoint =
                serde_json::from_str(&serialized).unwrap();
            assert_eq!(reloaded.reason, checkpoint.reason);
            assert!(
                checkpoint
                    .reason
                    .contains(if repeated { "repeated" } else { "step budget" })
            );
        }
    }
    let legacy: Value = json!({"id":uuid::Uuid::new_v4().to_string(),"title":"Legacy","workspace":"","model":"local","updated":"","entries":[]});
    let session: vmt_local_agent::types::Session = serde_json::from_value(legacy).unwrap();
    assert!(session.project_id.is_none() && session.checkpoint.is_none() && !session.archived);
}
