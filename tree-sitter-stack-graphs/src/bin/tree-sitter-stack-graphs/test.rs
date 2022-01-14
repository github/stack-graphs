// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use anyhow::Context as _;
use anyhow::Result;
use stack_graphs::graph::StackGraph;
use std::path::Path;
use std::path::PathBuf;
use tree_sitter_graph::ExecutionError;
use tree_sitter_graph::Variables;
use tree_sitter_stack_graphs::StackGraphLanguage;
use walkdir::WalkDir;

use crate::loader::LoaderArgs;

/// Run tests
#[derive(clap::Parser)]
pub struct Command {
    #[clap(flatten)]
    loader: LoaderArgs,

    /// Source paths to analyze.
    #[clap(name = "PATHS")]
    sources: Vec<PathBuf>,
}

impl Command {
    pub fn run(&self) -> Result<()> {
        let mut loader = self.loader.new_loader()?;
        for source_path in &self.sources {
            for source_entry in WalkDir::new(source_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let source_path = source_entry.path();

                match loader.load_for_source_path(source_path) {
                    Ok(sgl) => {
                        eprintln!("Process {}", source_path.display());
                        if let Err(e) = self.process(sgl, source_path) {
                            eprintln!("{:?}", e);
                            eprintln!("Failed {}", source_path.display());
                        }
                    }
                    Err(e) => {
                        eprintln!("{:?}", e);
                        eprintln!("Ignored {}", source_path.display());
                    }
                }
            }
        }
        Ok(())
    }

    fn process(&self, sgl: &mut StackGraphLanguage, source_path: &Path) -> Result<()> {
        let source = std::fs::read(source_path)
            .with_context(|| format!("Error reading source file {}", source_path.display()))?;
        let source = String::from_utf8(source)?;

        let mut globals = Variables::new();
        globals
            .add(
                "FILE_PATH".into(),
                source_path.as_os_str().to_str().unwrap().into(),
            )
            .map_err(|_| ExecutionError::DuplicateVariable("FILE_PATH".into()))?;

        let mut stack_graph = StackGraph::new();
        let file = stack_graph.get_or_create_file(source_path.to_str().unwrap());

        sgl.build_stack_graph_into(&mut stack_graph, file, &source, &mut globals)
            .with_context(|| {
                anyhow!(
                    "Could not execute stack graph rules on {}",
                    source_path.display()
                )
            })?;

        Ok(())
    }
}
