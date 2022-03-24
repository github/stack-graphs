// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use anyhow::Context as _;
use clap::ArgEnum;
use colored::Colorize as _;
use stack_graphs::graph::StackGraph;
use stack_graphs::paths::Paths;
use std::path::Path;
use std::path::PathBuf;
use tree_sitter_graph::parse_error::TreeWithParseErrorVec;
use tree_sitter_graph::Variables;
use tree_sitter_stack_graphs::loader::Loader;
use tree_sitter_stack_graphs::test::Test;
use tree_sitter_stack_graphs::test::TestFile;
use tree_sitter_stack_graphs::test::TestResult;
use tree_sitter_stack_graphs::LoadError;
use tree_sitter_stack_graphs::StackGraphLanguage;
use walkdir::WalkDir;

use crate::loader::LoaderArgs;
use crate::MAX_PARSE_ERRORS;

/// Flag to control output
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum OutputMode {
    Always,
    OnFailure,
}

impl OutputMode {
    fn test(&self, failure: bool) -> bool {
        match self {
            Self::Always => true,
            Self::OnFailure => failure,
        }
    }
}

/// Run tests
#[derive(clap::Parser)]
pub struct Command {
    #[clap(flatten)]
    loader: LoaderArgs,

    /// Source paths to test.
    #[clap(name = "PATHS")]
    #[clap(required = true)]
    sources: Vec<PathBuf>,

    /// Hide failure errors.
    #[clap(long)]
    hide_failure_errors: bool,

    /// Show ignored files in output.
    #[clap(long)]
    show_ignored: bool,

    /// Save graph.
    #[clap(short = 'G')]
    #[clap(long)]
    save_graph: bool,

    /// Save paths.
    #[clap(short = 'P')]
    #[clap(long)]
    save_paths: bool,

    /// Controls when graphs, paths, or visualization are saved.
    #[clap(long)]
    #[clap(arg_enum)]
    #[clap(default_value_t = OutputMode::OnFailure)]
    output_mode: OutputMode,
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
        let source = String::from_utf8(std::fs::read(source_path)?)?;
        let sgl = match loader.load_for_file(source_path, Some(&source))? {
            Some(sgl) => sgl,
            None => {
                if self.show_ignored {
                    println!("{} {}", "⦵".dimmed(), source_path.display());
                }
                return Ok(0);
            }
        };
        let mut test = Test::from_source(&source_path.to_string_lossy(), &source)?;
        for test_file in &test.files {
            let test_path = Path::new(test.graph[test_file.file].name());
            if source_path.extension() != test_path.extension() {
                return Err(anyhow!(
                    "Test file {} has different file extension than containing file {}.",
                    test_path.display(),
                    source_path.display()
                ));
            }
            self.build_file_stack_graph_into(source_path, sgl, test_file, &mut test.graph)?;
        }
        let result = test.run();
        let success = self.handle_result(source_path, &result)?;
        if self.output_mode.test(!success) {
            self.save_output(source_path, &test.graph, &mut test.paths)?;
        }
        Ok(result.failure_count())
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
        match sgl.build_stack_graph_into(graph, test_file.file, &test_file.source, &mut globals) {
            Err(LoadError::ParseErrors(parse_errors)) => {
                Err(self.map_parse_errors(source_path, &parse_errors, &test_file.source))
            }
            Err(e) => Err(e.into()),
            Ok(_) => Ok(()),
        }
    }

    fn map_parse_errors(
        &self,
        source_path: &Path,
        parse_errors: &TreeWithParseErrorVec,
        source: &str,
    ) -> anyhow::Error {
        let mut error = String::new();
        let parse_errors = parse_errors.errors();
        for parse_error in parse_errors.iter().take(MAX_PARSE_ERRORS) {
            let line = parse_error.node().start_position().row;
            let column = parse_error.node().start_position().column;
            error.push_str(&format!(
                "  {}:{}:{}: {}\n",
                source_path.display(),
                line + 1,
                column + 1,
                parse_error.display(&source, false)
            ));
        }
        if parse_errors.len() > MAX_PARSE_ERRORS {
            let more_errors = parse_errors.len() - MAX_PARSE_ERRORS;
            error.push_str(&format!(
                "  {} more parse error{} omitted\n",
                more_errors,
                if more_errors > 1 { "s" } else { "" },
            ));
        }
        anyhow!(error)
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
        if self.save_graph {
            let path = source_path.with_extension("graph.json");
            self.save_graph(&path, &graph)?;
            println!("  Graph: {}", path.display());
        }
        if self.save_paths {
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
}
