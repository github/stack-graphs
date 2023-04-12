use anyhow::anyhow;
use std::path::PathBuf;
use tree_sitter_stack_graphs::{ci::Tester, NoCancellation};

fn main() -> anyhow::Result<()> {
    let lc = match tree_sitter_stack_graphs_java::try_language_configuration(&NoCancellation) {
        Ok(lc) => lc,
        Err(err) => {
            eprintln!("{}", err.display_pretty());
            return Err(anyhow!("Language configuration error"));
        }
    };
    let test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test");
    Tester::new(vec![lc], vec![test_path]).run()
}
