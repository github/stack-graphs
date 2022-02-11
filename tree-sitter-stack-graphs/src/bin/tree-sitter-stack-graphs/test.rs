// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use anyhow::Context as _;
use colored::Colorize as _;
use stack_graphs::graph::StackGraph;
use stack_graphs::paths::Paths;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
use tree_sitter_graph::Variables;
use tree_sitter_stack_graphs::assert::Assertions;
use tree_sitter_stack_graphs::assert::Result;
use tree_sitter_stack_graphs::StackGraphLanguage;
use walkdir::WalkDir;

use crate::loader::LoaderArgs;

/// Run tests
#[derive(clap::Parser)]
pub struct Command {
    #[clap(flatten)]
    loader: LoaderArgs,

    /// Source paths to test.
    #[clap(name = "PATHS")]
    sources: Vec<PathBuf>,

    /// Hide failure errors.
    #[clap(long)]
    hide_failure_errors: bool,

    /// Save graph for failed tests.
    #[clap(long)]
    #[clap(short = 'G')]
    save_graph_on_failure: bool,

    /// Save paths for failed tests.
    #[clap(long)]
    #[clap(short = 'P')]
    save_paths_on_failure: bool,
}

impl Command {
    pub fn run(&self) -> anyhow::Result<()> {
        let mut loader = self.loader.new_loader()?;
        let mut total_failure_count = 0;
        for source_path in &self.sources {
            for source_entry in WalkDir::new(source_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let source_path = source_entry.path();

                match loader.load_for_source_path(source_path) {
                    Ok(sgl) => match self.process(sgl, source_path) {
                        Err(TestError::AssertionsFailed(failure_count)) => {
                            total_failure_count += failure_count;
                        }
                        r => r.with_context(|| {
                            format!("Error reading test file {}", source_path.display())
                        })?,
                    },
                    Err(e) => {
                        println!("{} {}", "⦵".dimmed(), source_path.display(),);
                        if !self.hide_failure_errors {
                            Self::print_err(e);
                        }
                    }
                }
            }
        }
        if total_failure_count == 0 {
            Ok(())
        } else {
            Err(anyhow!("{} assertions failed", total_failure_count))
        }
    }

    fn process(
        &self,
        sgl: &mut StackGraphLanguage,
        source_path: &Path,
    ) -> std::result::Result<(), TestError> {
        let source = std::fs::read(source_path).map_err(TestError::other)?;
        let source = String::from_utf8(source).map_err(TestError::other)?;

        let mut globals = Variables::new();
        globals
            .add(
                "FILE_PATH".into(),
                source_path.as_os_str().to_str().unwrap().into(),
            )
            .expect("Failed to set FILE_PATH");

        let mut stack_graph = StackGraph::new();
        let file = stack_graph.get_or_create_file(source_path.to_str().unwrap());

        sgl.build_stack_graph_into(&mut stack_graph, file, &source, &mut globals)
            .map_err(TestError::other)?;

        let assertions = Assertions::from_source(file, &source).map_err(TestError::other)?;
        let mut paths = Paths::new();
        let result = assertions.run(&stack_graph, &mut paths);
        if result.failure_count() == 0 {
            println!(
                "{} {}: {}/{} assertions",
                "✓".green(),
                stack_graph[file],
                result.success_count(),
                assertions.count()
            );
            Ok(())
        } else {
            println!(
                "{} {}: {}/{} assertions",
                "✗".red(),
                stack_graph[file],
                result.success_count(),
                assertions.count()
            );
            if !self.hide_failure_errors {
                for result in &result {
                    if let Result::Failure(e) = result {
                        println!("  {}", e);
                    }
                }
            }
            let graph_path = source_path.with_extension("graph.json");
            let paths_path = source_path.with_extension("paths.json");
            if self.save_graph_on_failure {
                let json = stack_graph
                    .to_json_string_pretty()
                    .map_err(TestError::other)?;
                std::fs::write(&graph_path, json).expect("Unable to write graph");
                println!("  Graph: {}", graph_path.display());
            }
            if self.save_paths_on_failure {
                let json = paths
                    .to_json_string_pretty(&stack_graph)
                    .map_err(TestError::other)?;
                std::fs::write(&paths_path, json).expect("Unable to write paths");
                println!("  Paths: {}", paths_path.display());
            }
            Err(TestError::AssertionsFailed(result.failure_count()))
        }
    }

    fn print_err<E>(err: E)
    where
        E: Into<anyhow::Error>,
    {
        let err = err.into();
        println!("  {}", err);
        let mut first = true;
        for err in err.chain().skip(1) {
            if first {
                println!("  Caused by:");
                first = false;
            }
            println!("      {}", err);
        }
    }
}

#[derive(Debug, Error)]
pub enum TestError {
    #[error("{0} assertions failed")]
    AssertionsFailed(usize),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl TestError {
    fn other<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Other(error.into())
    }
}
