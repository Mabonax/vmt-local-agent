fn main() -> anyhow::Result<()> {
    let model = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "deepseek-r1-local:7b".into());
    let selected = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "qwen3.5:4b".into());
    match model.as_str() {
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
