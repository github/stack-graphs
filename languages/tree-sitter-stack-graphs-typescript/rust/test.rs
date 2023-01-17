// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::path::PathBuf;
use tree_sitter_stack_graphs::ci::Tester;
use tree_sitter_stack_graphs::NoCancellation;

fn main() -> anyhow::Result<()> {
    let test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test");
    Tester::new(
        vec![tree_sitter_stack_graphs_typescript::language_configuration(
            &NoCancellation,
        )],
        vec![test_path],
    )
    .run()
}
