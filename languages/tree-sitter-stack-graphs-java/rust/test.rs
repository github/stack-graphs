use std::path::PathBuf;
use tree_sitter_stack_graphs::cli::CiTester;

fn main() -> anyhow::Result<()> {
    let test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    CiTester::new(
        vec![tree_sitter_stack_graphs_java::language_configuration()],
        vec![test_path],
    )
    .run()
}
