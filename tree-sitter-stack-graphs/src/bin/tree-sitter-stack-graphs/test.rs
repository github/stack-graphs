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
use tree_sitter_stack_graphs::test::TestFragment;
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
    #[clap(value_name = "PATHS", required = true)]
    sources: Vec<PathBuf>,

    /// Hide passing tests.
    #[clap(long)]
    hide_passing: bool,

    /// Hide failure error details.
    #[clap(long)]
    hide_failure_errors: bool,

    /// Show ignored files in output.
    #[clap(long)]
    show_ignored: bool,

    /// Save graph.
    #[clap(long, short = 'G', min_values = 0, max_values = 1)]
    save_graph: Option<Vec<String>>,

    /// Save paths.
    #[clap(long, short = 'P', min_values = 0, max_values = 1)]
    save_paths: Option<Vec<String>>,

    /// Save visualization.
    #[clap(long, short = 'V', min_values = 0, max_values = 1)]
    save_visualization: Option<Vec<String>>,

    /// Controls when graphs, paths, or visualization are saved.
    #[clap(long, arg_enum, default_value_t = OutputMode::OnFailure)]
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
        let mut test = Test::from_source(&source_path, &source)?;
        for test_fragment in &test.fragments {
            let test_path = Path::new(test.graph[test_fragment.file].name()).to_path_buf();
            if source_path.extension() != test_path.extension() {
                return Err(anyhow!(
                    "Test fragment {} has different file extension than test file {}",
                    test_path.display(),
                    source_path.display()
                ));
            }
            self.build_fragment_stack_graph_into(&test_path, sgl, test_fragment, &mut test.graph)?;
        }
        let result = test.run();
        let success = self.handle_result(source_path, &result)?;
        if self.output_mode.test(!success) {
            self.save_output(source_path, &test.graph, &mut test.paths, success)?;
        }
        Ok(result.failure_count())
    }

    fn build_fragment_stack_graph_into(
        &self,
        source_path: &Path,
        sgl: &mut StackGraphLanguage,
        test_fragment: &TestFragment,
        graph: &mut StackGraph,
    ) -> anyhow::Result<()> {
        let mut globals = Variables::new();
        match sgl.build_stack_graph_into(
            graph,
            test_fragment.file,
            &test_fragment.source,
            &mut globals,
        ) {
            Err(LoadError::ParseErrors(parse_errors)) => {
                Err(self.map_parse_errors(source_path, &parse_errors, &test_fragment.source))
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
        if !success || !self.hide_passing {
            println!(
                "{} {}: {}/{} assertions",
                if success { "✓".green() } else { "✗".red() },
                source_path.display(),
                result.success_count(),
                result.count()
            );
        }
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
        success: bool,
    ) -> anyhow::Result<()> {
        let root = source_path
            .parent()
            .map(|p| p.to_string_lossy().into_owned());
        let dirs = Some(PathBuf::default().to_string_lossy().into_owned());
        let name = source_path
            .file_stem()
            .map(|p| p.to_string_lossy().into_owned());
        let ext = source_path
            .extension()
            .map(|p| format!(".{}", p.to_string_lossy().into_owned()));

        if let Some(path) = Self::output_path(
            self.save_graph.as_ref(),
            "%n.graph.json",
            root.as_deref(),
            dirs.as_deref(),
            name.as_deref(),
            ext.as_deref(),
        ) {
            self.save_graph(&path, &graph)?;
            if !success || !self.hide_passing {
                println!("  Graph: {}", path.display());
            }
        }
        if let Some(path) = Self::output_path(
            self.save_paths.as_ref(),
            "%n.paths.json",
            root.as_deref(),
            dirs.as_deref(),
            name.as_deref(),
            ext.as_deref(),
        ) {
            self.save_paths(&path, paths, graph)?;
            if !success || !self.hide_passing {
                println!("  Paths: {}", path.display());
            }
        }
        if let Some(path) = Self::output_path(
            self.save_visualization.as_ref(),
            "%n.html",
            root.as_deref(),
            dirs.as_deref(),
            name.as_deref(),
            ext.as_deref(),
        ) {
            self.save_visualization(&path, paths, graph, &source_path)?;
            if !success || !self.hide_passing {
                println!("  Visualization: {}", path.display());
            }
        }
        Ok(())
    }

    fn output_path(
        flag: Option<&Vec<String>>,
        default: &str,
        root: Option<&str>,
        dirs: Option<&str>,
        name: Option<&str>,
        ext: Option<&str>,
    ) -> Option<PathBuf> {
        flag.map(|ps| {
            Self::format_path(
                ps.iter().next().map_or(default, |p| &p),
                root,
                dirs,
                name,
                ext,
            )
        })
    }

    fn format_path(
        format: &str,
        root: Option<&str>,
        dirs: Option<&str>,
        name: Option<&str>,
        ext: Option<&str>,
    ) -> PathBuf {
        let mut path = String::new();
        let mut in_placeholder = false;
        for c in format.chars() {
            if in_placeholder {
                in_placeholder = false;
                match c {
                    'r' => {
                        if let Some(root) = root {
                            path.push_str(root)
                        }
                    }
                    'n' => {
                        if let Some(name) = name {
                            path.push_str(name)
                        }
                    }
                    'd' => {
                        if let Some(dirs) = dirs {
                            path.push_str(dirs)
                        }
                    }
                    'e' => {
                        if let Some(ext) = ext {
                            path.push_str(ext)
                        }
                    }
                    '%' => path.push('%'),
                    c => panic!("Unsupported placeholder '%{}'", c),
                }
            } else if c == '%' {
                in_placeholder = true;
            } else {
                path.push(c);
            }
        }
        if in_placeholder {
            panic!("Unsupported '%' at end");
        }
        path.into()
    }

    fn save_graph(&self, path: &Path, graph: &StackGraph) -> anyhow::Result<()> {
        let json = graph.to_json().to_string_pretty()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, json).expect("Unable to write graph");
        Ok(())
    }

    fn save_paths(&self, path: &Path, paths: &mut Paths, graph: &StackGraph) -> anyhow::Result<()> {
        let json = paths.to_json(graph, |_, _, _| true).to_string_pretty()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, json).expect("Unable to write paths");
        Ok(())
    }

    fn save_visualization(
        &self,
        path: &Path,
        paths: &mut Paths,
        graph: &StackGraph,
        source_path: &Path,
    ) -> anyhow::Result<()> {
        let html = graph.to_html_string(paths, &format!("{}", source_path.display()))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, html).expect("Unable to write visualization");
        Ok(())
    }
}
