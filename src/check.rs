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
