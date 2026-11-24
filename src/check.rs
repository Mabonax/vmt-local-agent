fn main() -> anyhow::Result<()> {
    let raw: Vec<String> = std::env::args().collect();
    if let Some(index) = raw.iter().position(|value| value == "--data-dir") {
        vmt_local_agent::store::configure_data_dir(
            raw.get(index + 1)
                .ok_or_else(|| anyhow::anyhow!("Missing profile directory"))?
                .into(),
        )?;
    }
    let model = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "deepseek-r1-local:7b".into());
    let selected = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "qwen3.5:4b".into());
    match model.as_str() {
        "--unified-live" => {
            let project = raw.get(3).ok_or_else(|| {
                anyhow::anyhow!("Provide model, synthetic workspace and request file")
            })?;
            let request = raw
                .get(4)
                .ok_or_else(|| anyhow::anyhow!("Provide request file"))?;
            return vmt_local_agent::unified_trial::run(
                &selected,
                project,
                request,
                raw.iter().any(|v| v == "--json"),
            );
        }
        "--tool-pack-run" | "--tool-pack-batch" => {
            let request_file = raw.get(3).ok_or_else(|| {
                anyhow::anyhow!("Provide workspace and a reviewed request JSON file")
            })?;
            anyhow::ensure!(
                std::fs::metadata(request_file)?.len() <= 1_000_000,
                "Request file exceeds limit"
            );
            let request: serde_json::Value = serde_json::from_slice(&std::fs::read(request_file)?)?;
            let web = raw.iter().any(|v| v == "--internet");
            let mut tools = vmt_local_agent::tools::Tools {
                workspace: vmt_local_agent::workspace::Workspace::new(&selected)?,
                observed: Default::default(),
            };
            let requests = if model == "--tool-pack-batch" {
                request
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Expected reviewed request array"))?
                    .clone()
            } else {
                vec![request]
            };
            anyhow::ensure!(requests.len() <= 20, "Batch exceeds 20 reviewed actions");
            let mut results = Vec::new();
            for request in requests {
                let vmt_local_agent::tools::Prepared::Review(proposal) =
                    tools.prepare("tool_pack_call", &request, false, web)?
                else {
                    anyhow::bail!("Expected reviewed capability")
                };
                // Explicit CLI operator invocation of the exact immutable parsed file.
                // This entry point is not registered as a model capability.
                if raw.iter().any(|v| v == "--review-stdin") {
                    println!("{}", vmt_local_agent::tools::diff(&proposal));
                    println!("Prepared exact action. Type APPROVE to execute.");
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    let mut answer = String::new();
                    std::io::stdin().read_line(&mut answer)?;
                    anyhow::ensure!(answer.trim() == "APPROVE", "Action denied");
                }
                let mut journal =
                    vmt_local_agent::task_journal::TaskJournal::start(&tools.workspace)?;
                let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                if let Some(index) = raw.iter().position(|v| v == "--cancel-after-ms") {
                    let delay: u64 = raw
                        .get(index + 1)
                        .ok_or_else(|| anyhow::anyhow!("Missing cancellation delay"))?
                        .parse()?;
                    anyhow::ensure!(delay <= 120000, "Cancellation delay exceeds trial bound");
                    let stop = cancel.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(delay));
                        stop.store(true, std::sync::atomic::Ordering::Relaxed);
                    });
                }
                let result = vmt_local_agent::execution::execute_reviewed(
                    &mut tools,
                    "tool_pack_call",
                    &proposal,
                    false,
                    web,
                    &mut journal,
                    &cancel,
                );
                journal.finish(if result.is_ok() {
                    "completed"
                } else {
                    "failed"
                })?;
                results.push(serde_json::from_str::<serde_json::Value>(&result?)?);
            }
            if model == "--tool-pack-batch" {
                println!("{}", serde_json::to_string(&results)?);
            } else {
                println!("{}", results[0]);
            }
            return Ok(());
        }
        "--tool-pack-save" => {
            let id = raw.get(3).ok_or_else(|| {
                anyhow::anyhow!("Provide workspace, artifact ID and new destination")
            })?;
            let destination = raw
                .get(4)
                .ok_or_else(|| anyhow::anyhow!("Provide destination"))?;
            vmt_local_agent::tool_packs::save_artifact(
                &vmt_local_agent::workspace::Workspace::new(&selected)?,
                id,
                std::path::Path::new(destination),
            )?;
            println!("Verified artifact saved to the explicitly selected new file.");
            return Ok(());
        }
        "--tool-pack-configure" => {
            let office = raw.get(3).map(String::as_str).filter(|s| *s != "none");
            let config = vmt_local_agent::tool_packs::configure(&selected, office)?;
            println!(
                "Reviewed tool runtime configured: {} capabilities, discovery budget {} bytes. Actions still require review.",
                vmt_local_agent::tool_packs::catalog().len(),
                config.discovery_bytes
            );
            return Ok(());
        }
        "--tool-pack-status" => {
            let workspace = vmt_local_agent::workspace::Workspace::new(&selected)?;
            println!("{}", vmt_local_agent::tool_packs::status(&workspace)?);
            return Ok(());
        }
        "--document-authoring" => {
            let project = raw.get(3).ok_or_else(|| anyhow::anyhow!("Provide model, project, request file, stage 1-3 and optional output-token limit"))?;
            let request = raw
                .get(4)
                .ok_or_else(|| anyhow::anyhow!("Provide request file"))?;
            let stage = raw
                .get(5)
                .ok_or_else(|| anyhow::anyhow!("Provide stage 1-3"))?
                .parse()?;
            let output = raw.get(6).map(|v| v.parse()).transpose()?.unwrap_or(2048);
            return vmt_local_agent::document_trial::run(
                &selected, project, request, stage, output,
            );
        }
        "--creative-live" => {
            let project = raw.get(3).ok_or_else(|| {
                anyhow::anyhow!("Provide model, fixture project and native/json protocol")
            })?;
            let protocol = raw.get(4).map(String::as_str).unwrap_or("native");
            return vmt_local_agent::verification::creative_live_check(
                &selected, project, protocol,
            );
        }
        "--document-task" | "--document-collision-task" => {
            let input = std::env::args()
                .nth(3)
                .ok_or_else(|| anyhow::anyhow!("Provide model and original DOCX path"))?;
            return vmt_local_agent::verification::document_task_check_case(
                &selected,
                &input,
                model == "--document-collision-task",
            );
        }
        "--design-live" => {
            let source = std::env::args()
                .nth(3)
                .ok_or_else(|| anyhow::anyhow!("Provide model and design skills root"))?;
            return vmt_local_agent::verification::design_skills_check(&selected, &source);
        }
        "--design-configure" => {
            let soffice = std::env::args().nth(3).ok_or_else(|| {
                anyhow::anyhow!("Provide Python and LibreOffice executable paths")
            })?;
            vmt_local_agent::design_tools::configure(&selected, &soffice)?;
            println!("Design runtime configured. Individual tool actions still require review.");
            return Ok(());
        }
        "--design-operation" => {
            let workspace = std::env::args()
                .nth(3)
                .ok_or_else(|| anyhow::anyhow!("Provide request JSON and workspace"))?;
            let args: serde_json::Value = serde_json::from_str(&selected)?;
            let mut tools = vmt_local_agent::tools::Tools {
                workspace: vmt_local_agent::workspace::Workspace::new(workspace)?,
                observed: Default::default(),
            };
            let vmt_local_agent::tools::Prepared::Review(proposal) =
                tools.prepare("design_tool", &args, false, false)?
            else {
                anyhow::bail!("Expected reviewed design proposal")
            };
            let mut journal = vmt_local_agent::task_journal::TaskJournal::start(&tools.workspace)?;
            let result = vmt_local_agent::execution::execute_reviewed(
                &mut tools,
                "design_tool",
                &proposal,
                false,
                false,
                &mut journal,
                &std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            );
            journal.finish(if result.is_ok() {
                "completed"
            } else {
                "failed"
            })?;
            println!("{}", result?);
            return Ok(());
        }
        "--skills-import" => {
            let library =
                vmt_local_agent::portable_skills::inspect_library(std::path::Path::new(&selected))?;
            let count = library.skills.len();
            vmt_local_agent::portable_skills::Registry::load()?.approve(library)?;
            println!(
                "Approved {count} portable guidance skills. No scripts executed or runtime permissions granted."
            );
            return Ok(());
        }
        "--skills-audit" => {
            for row in vmt_local_agent::skills::portable_rows()? {
                println!(
                    "{} | {} | version {} | license {} | {}",
                    row.id, row.status, row.version, row.license, row.fingerprint
                );
            }
            return Ok(());
        }
        "--skills-catalog" => {
            println!("{}", vmt_local_agent::skills::catalog());
            return Ok(());
        }
        "--permissions-skills" => {
            let source = std::env::args()
                .nth(3)
                .ok_or_else(|| anyhow::anyhow!("Provide model and a skills repository path"))?;
            return vmt_local_agent::verification::permission_skills_check(&selected, &source);
        }
        "--mcp-fixture" => return vmt_local_agent::verification::mcp_fixture(),
        "--craft" => return vmt_local_agent::verification::craft(),
        "--scenario" => return vmt_local_agent::verification::scenario(&selected),
        "--runtime" => return vmt_local_agent::verification::runtime(),
        "--resume" => return vmt_local_agent::verification::resume(),
        "--rag" => return vmt_local_agent::verification::rag(),
        "--research" => return vmt_local_agent::verification::research_check(),
        "--project-context" => return vmt_local_agent::verification::project_context(&selected),
        "--chat" => {
            return vmt_local_agent::verification::model_check(
                &selected,
                "chat",
                "auto",
                "What is 17 multiplied by 23? Reply with the result.",
                "chat",
            );
        }
        "--json" => {
            return vmt_local_agent::verification::model_check(
                &selected,
                "agent",
                "json",
                "Read hello.txt and report its exact contents. Do not change files or run commands.",
                "json",
            );
        }
        _ => {}
    }
    if model == "--tools" {
        return vmt_local_agent::tools::acceptance_tools();
    }
    if model == "--edit" {
        return vmt_local_agent::agent::smoke_edit_test(
            &std::env::args()
                .nth(2)
                .unwrap_or_else(|| "deepseek-r1-local:7b".into()),
        );
    }
    vmt_local_agent::agent::smoke_test(&model)
}
