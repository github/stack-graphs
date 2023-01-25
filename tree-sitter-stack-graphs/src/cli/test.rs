// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use anyhow::Context as _;
use clap::ArgEnum;
use clap::Args;
use clap::ValueHint;
use colored::Colorize as _;
use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use stack_graphs::json::Filter;
use stack_graphs::paths::Paths;
use std::path::Path;
use std::path::PathBuf;
use tree_sitter_graph::Variables;
use walkdir::WalkDir;

use crate::cli::util::map_parse_errors;
use crate::cli::util::path_exists;
use crate::cli::util::PathSpec;
use crate::loader::FileReader;
use crate::loader::LanguageConfiguration;
use crate::loader::Loader;
use crate::test::Test;
use crate::test::TestResult;
use crate::LoadError;
use crate::NoCancellation;
use crate::StackGraphLanguage;

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
#[derive(Args)]
#[clap(after_help = r#"PATH SPECIFICATIONS:
    Output filenames can be specified using placeholders based on the input file.
    The following placeholders are supported:
         %r   the root path, which is the directory argument which contains the file,
              or the directory of the file argument
         %d   the path directories relative to the root
         %n   the name of the file
         %e   the file extension (including the preceding dot)
         %%   a literal percentage sign

    Empty directory placeholders (%r and %d) are replaced by "." so that the shape
    of the path is not accidently changed. For example, "test -V %d/%n.html mytest.py"
    results in "./mytest.html" instead of the unintented "/mytest.html".

    Note that on Windows the path specification must be valid Unicode, but all valid
    paths (including ones that are not valid Unicode) are accepted as arguments, and
    placeholders are correctly subtituted for all paths.
"#)]
pub struct TestArgs {
    /// Test file or directory paths.
    #[clap(
        value_name = "TEST_PATH",
        required = true,
        value_hint = ValueHint::AnyPath,
        parse(from_os_str),
        validator_os = path_exists
    )]
    pub test_paths: Vec<PathBuf>,

    /// Hide passing tests.
    #[clap(long)]
    pub hide_passing: bool,

    /// Hide failure error details.
    #[clap(long)]
    pub hide_failure_errors: bool,

    /// Show ignored files in output.
    #[clap(long)]
    pub show_ignored: bool,

    /// Save graph for tests matching output mode.
    /// Takes an optional path specification argument for the output file.
    /// [default: %n.graph.json]
    #[clap(
        long,
        short = 'G',
        value_name = "PATH_SPEC",
        min_values = 0,
        max_values = 1,
        require_equals = true,
        default_missing_value = "%n.graph.json"
    )]
    pub save_graph: Option<PathSpec>,

    /// Save paths for tests matching output mode.
    /// Takes an optional path specification argument for the output file.
    /// [default: %n.paths.json]
    #[clap(
        long,
        short = 'P',
        value_name = "PATH_SPEC",
        min_values = 0,
        max_values = 1,
        require_equals = true,
        default_missing_value = "%n.paths.json"
    )]
    pub save_paths: Option<PathSpec>,

    /// Save visualization for tests matching output mode.
    /// Takes an optional path specification argument for the output file.
    /// [default: %n.html]
    #[clap(
        long,
        short = 'V',
        value_name = "PATH_SPEC",
        min_values = 0,
        max_values = 1,
        require_equals = true,
        default_missing_value = "%n.html"
    )]
    pub save_visualization: Option<PathSpec>,

    /// Controls when graphs, paths, or visualization are saved.
    #[clap(
        long,
        arg_enum,
        default_value_t = OutputMode::OnFailure,
        require_equals = true,
    )]
    pub output_mode: OutputMode,
}

impl TestArgs {
    pub fn new(test_paths: Vec<PathBuf>) -> Self {
        Self {
            test_paths,
            hide_passing: false,
            hide_failure_errors: false,
            show_ignored: false,
            save_graph: None,
            save_paths: None,
            save_visualization: None,
            output_mode: OutputMode::OnFailure,
        }
    }

    pub fn run(&self, loader: &mut Loader) -> anyhow::Result<()> {
        let mut total_result = TestResult::new();
        for test_path in &self.test_paths {
            if test_path.is_dir() {
                let test_root = test_path;
                for test_entry in WalkDir::new(test_root)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let test_path = test_entry.path();
                    let test_result = self.run_test_with_context(test_root, test_path, loader)?;
                    total_result.absorb(test_result);
                }
            } else {
                let test_root = test_path.parent().unwrap();
                let test_result = self.run_test_with_context(test_root, test_path, loader)?;
                total_result.absorb(test_result);
            }
        }

        if total_result.failure_count() > 0 {
            return Err(anyhow!(total_result.to_string()));
        }

        Ok(())
    }

    /// Run test file and add error context to any failures that are returned.
    fn run_test_with_context(
        &self,
        test_root: &Path,
        test_path: &Path,
        loader: &mut Loader,
    ) -> anyhow::Result<TestResult> {
        self.run_test(test_root, test_path, loader)
            .with_context(|| format!("Error running test {}", test_path.display()))
    }

    /// Run test file.
    fn run_test(
        &self,
        test_root: &Path,
        test_path: &Path,
        loader: &mut Loader,
    ) -> anyhow::Result<TestResult> {
        let mut file_reader = FileReader::new();
        let lc = match loader.load_for_file(test_path, &mut file_reader, &NoCancellation)? {
            Some(sgl) => sgl,
            None => {
                if self.show_ignored {
                    println!("{} {}", "⦵".dimmed(), test_path.display());
                }
                return Ok(TestResult::new());
            }
        };
        let source = file_reader.get(test_path)?;
        let default_fragment_path = test_path.strip_prefix(test_root).unwrap();
        let mut test = Test::from_source(&test_path, &source, default_fragment_path)?;
        self.load_builtins_into(&lc, &mut test.graph)
            .with_context(|| format!("Loading builtins into {}", test_path.display()))?;
        let mut globals = Variables::new();
        for test_fragment in &test.fragments {
            if let Some(fa) = test_fragment
                .path
                .file_name()
                .and_then(|f| lc.special_files.get(&f.to_string_lossy()))
            {
                let mut all_paths = test.fragments.iter().map(|f| f.path.as_path());
                fa.build_stack_graph_into(
                    &mut test.graph,
                    test_fragment.file,
                    &test_fragment.path,
                    &test_fragment.source,
                    &mut all_paths,
                    &test_fragment.globals,
                    &NoCancellation,
                )?;
            } else if lc.matches_file(
                &test_fragment.path,
                &mut Some(test_fragment.source.as_ref()),
            )? {
                globals.clear();
                test_fragment.add_globals_to(&mut globals);
                self.build_fragment_stack_graph_into(
                    &test_fragment.path,
                    &lc.sgl,
                    test_fragment.file,
                    &test_fragment.source,
                    &globals,
                    &mut test.graph,
                )?;
            } else {
                return Err(anyhow!(
                    "Test fragment {} not supported by language of test file {}",
                    test_fragment.path.display(),
                    test.path.display()
                ));
            }
        }
        let result = test.run(&NoCancellation)?;
        let success = self.handle_result(test_path, &result)?;
        if self.output_mode.test(!success) {
            let files = test.fragments.iter().map(|f| f.file).collect::<Vec<_>>();
            self.save_output(
                test_root,
                test_path,
                &test.graph,
                &mut test.paths,
                &|_: &StackGraph, h: &Handle<File>| files.contains(h),
                success,
            )?;
        }
        Ok(result)
    }

    fn load_builtins_into(
        &self,
        lc: &LanguageConfiguration,
        graph: &mut StackGraph,
    ) -> anyhow::Result<()> {
        if let Err(h) = graph.add_from_graph(&lc.builtins) {
            return Err(anyhow!("Duplicate builtin file {}", &graph[h]));
        }
        Ok(())
    }

    fn build_fragment_stack_graph_into(
        &self,
        test_path: &Path,
        sgl: &StackGraphLanguage,
        file: Handle<File>,
        source: &str,
        globals: &Variables,
        graph: &mut StackGraph,
    ) -> anyhow::Result<()> {
        match sgl.build_stack_graph_into(graph, file, source, globals, &NoCancellation) {
            Err(LoadError::ParseErrors(parse_errors)) => {
                Err(map_parse_errors(test_path, &parse_errors, source, "  "))
            }
            Err(e) => Err(e.into()),
            Ok(_) => Ok(()),
        }
    }

    fn handle_result(&self, test_path: &Path, result: &TestResult) -> anyhow::Result<bool> {
        let success = result.failure_count() == 0;
        if !success || !self.hide_passing {
            println!(
                "{} {}: {}/{} assertions",
                if success { "✓".green() } else { "✗".red() },
                test_path.display(),
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
        test_root: &Path,
        test_path: &Path,
        graph: &StackGraph,
        paths: &mut Paths,
        filter: &dyn Filter,
        success: bool,
    ) -> anyhow::Result<()> {
        if let Some(path) = self
            .save_graph
            .as_ref()
            .map(|spec| spec.format(test_root, test_path))
        {
            self.save_graph(&path, &graph, filter)?;
            if !success || !self.hide_passing {
                println!("  Graph: {}", path.display());
            }
        }
        if let Some(path) = self
            .save_paths
            .as_ref()
            .map(|spec| spec.format(test_root, test_path))
        {
            self.save_paths(&path, paths, graph, filter)?;
            if !success || !self.hide_passing {
                println!("  Paths: {}", path.display());
            }
        }
        if let Some(path) = self
            .save_visualization
            .as_ref()
            .map(|spec| spec.format(test_root, test_path))
        {
            self.save_visualization(&path, paths, graph, filter, &test_path)?;
            if !success || !self.hide_passing {
                println!("  Visualization: {}", path.display());
            }
        }
        Ok(())
    }

    fn save_graph(
        &self,
        path: &Path,
        graph: &StackGraph,
        filter: &dyn Filter,
    ) -> anyhow::Result<()> {
        let json = graph.to_json(filter).to_string_pretty()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, json)
            .with_context(|| format!("Unable to write graph {}", path.display()))?;
        Ok(())
    }

    fn save_paths(
        &self,
        path: &Path,
        paths: &mut Paths,
        graph: &StackGraph,
        filter: &dyn Filter,
    ) -> anyhow::Result<()> {
        let json = paths.to_json(graph, filter).to_string_pretty()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, json)
            .with_context(|| format!("Unable to write graph {}", path.display()))?;
        Ok(())
    }

    fn save_visualization(
        &self,
        path: &Path,
        paths: &mut Paths,
        graph: &StackGraph,
        filter: &dyn Filter,
        test_path: &Path,
    ) -> anyhow::Result<()> {
        let html = graph.to_html_string(&format!("{}", test_path.display()), paths, filter)?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, html)
            .with_context(|| format!("Unable to write graph {}", path.display()))?;
        Ok(())
    }
}
