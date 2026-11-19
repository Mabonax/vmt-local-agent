fn main() -> anyhow::Result<()> {
    let model = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "deepseek-r1-local:7b".into());
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
