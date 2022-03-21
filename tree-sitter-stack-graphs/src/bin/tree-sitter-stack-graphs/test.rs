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
use tree_sitter_graph::Variables;
use tree_sitter_stack_graphs::loader::Loader;
use tree_sitter_stack_graphs::test::Test;
use tree_sitter_stack_graphs::test::TestFile;
use tree_sitter_stack_graphs::test::TestResult;
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
        self.validate_source_paths()?;

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
                total_failure_count += self
                    .run_test(source_path, &mut loader)
                    .with_context(|| format!("Error running test {}", source_path.display()))?;
            }
        }

        if total_failure_count > 0 {
            return Err(anyhow!(
                "{} assertion{} failed",
                total_failure_count,
                if total_failure_count == 1 { "" } else { "s" }
            ));
        }

        Ok(())
    }

    fn validate_source_paths(&self) -> anyhow::Result<()> {
        if self.sources.is_empty() {
            return Err(anyhow!("No source paths provided"));
        }
        for source_path in &self.sources {
            if !source_path.exists() {
                return Err(anyhow!(
                    "Source path {} does not exist",
                    source_path.display()
                ));
            }
        }
        Ok(())
    }

    fn run_test(&self, source_path: &Path, loader: &mut Loader) -> anyhow::Result<usize> {
        let mut test = self.read_test(source_path)?;
        for test_file in &test.files {
            let test_path = Path::new(test.graph[test_file.file].name());
            let sgl = match loader.load_for_source_path(test_path) {
                Ok(sgl) => sgl,
                Err(e) => {
                    println!("{} {}", "⦵".dimmed(), source_path.display());
                    if !self.hide_failure_errors {
                        Self::print_err(e);
                    }
                    return Ok(0);
                }
            };
            self.build_file_stack_graph_into(source_path, sgl, test_file, &mut test.graph)?;
        }
        let result = test.run();
        let success = self.handle_result(source_path, &result)?;
        if !success {
            self.save_output(source_path, &test.graph, &mut test.paths)?;
        }
        Ok(result.failure_count())
    }

    fn read_test(&self, source_path: &Path) -> anyhow::Result<Test> {
        let source = std::fs::read(source_path)?;
        let source = String::from_utf8(source)?;
        let test = Test::from_source(&source_path.to_string_lossy(), &source)?;
        Ok(test)
    }

    fn build_file_stack_graph_into(
        &self,
        source_path: &Path,
        sgl: &mut StackGraphLanguage,
        test_file: &TestFile,
        graph: &mut StackGraph,
    ) -> anyhow::Result<()> {
        let mut globals = Variables::new();
        globals
            .add(
                "FILE_PATH".into(),
                format!("{}", source_path.display()).into(),
            )
            .expect("Failed to set FILE_PATH");
        sgl.build_stack_graph_into(graph, test_file.file, &test_file.source, &mut globals)?;
        Ok(())
    }

    fn handle_result(&self, source_path: &Path, result: &TestResult) -> anyhow::Result<bool> {
        let success = result.failure_count() == 0;
        println!(
            "{} {}: {}/{} assertions",
            if success { "✓".green() } else { "✗".red() },
            source_path.display(),
            result.success_count(),
            result.count()
        );
        if !success && !self.hide_failure_errors {
            for failure in result.failures_iter() {
                println!("  {}", failure);
            }
        }
        Ok(success)
    }

    fn save_output(
        &self,
        source_path: &Path,
        graph: &StackGraph,
        paths: &mut Paths,
    ) -> anyhow::Result<()> {
        if self.save_graph_on_failure {
            let path = source_path.with_extension("graph.json");
            self.save_graph(&path, &graph)?;
            println!("  Graph: {}", path.display());
        }
        if self.save_paths_on_failure {
            let path = source_path.with_extension("paths.json");
            self.save_paths(&path, paths, graph)?;
            println!("  Paths: {}", path.display());
        }
        Ok(())
    }

    fn save_graph(&self, path: &Path, graph: &StackGraph) -> anyhow::Result<()> {
        let json = graph.to_json().to_string_pretty()?;
        std::fs::write(&path, json).expect("Unable to write graph");
        Ok(())
    }

    fn save_paths(&self, path: &Path, paths: &mut Paths, graph: &StackGraph) -> anyhow::Result<()> {
        let json = paths.to_json(graph, |_, _, _| true).to_string_pretty()?;
        std::fs::write(&path, json).expect("Unable to write paths");
        Ok(())
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
