use std::path::PathBuf;
use tree_sitter_stack_graphs::{cli::CiTester, NoCancellation};

fn main() -> anyhow::Result<()> {
    let test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test");
    CiTester::new(
        vec![tree_sitter_stack_graphs_java::language_configuration(
            &NoCancellation,
        )],
        vec![test_path],
    )
    .run()
}
