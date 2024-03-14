// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use std::path::PathBuf;
use tree_sitter_stack_graphs::ci::Tester;
use tree_sitter_stack_graphs::NoCancellation;

fn main() -> anyhow::Result<()> {
    let lc_factories = [
        tree_sitter_stack_graphs_typescript::try_language_configuration_typescript,
        tree_sitter_stack_graphs_typescript::try_language_configuration_tsx,
    ];

    let lcs = lc_factories
        .iter()
        .map(|lc_factory| lc_factory(&NoCancellation))
        .collect::<Result<Vec<_>, _>>()
        .inspect_err(|err| eprintln!("{}", err.display_pretty()))
        .map_err(|_| anyhow!("Language configuration error"))?;

    let test_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test");
    Tester::new(lcs, vec![test_path]).run()
}
