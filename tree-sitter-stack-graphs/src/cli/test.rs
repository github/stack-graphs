// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use clap::Args;
use clap::ValueEnum;
use clap::ValueHint;
use itertools::Itertools;
use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;
use stack_graphs::partial::PartialPaths;
use stack_graphs::serde::Filter;
use stack_graphs::stitching::Database;
use stack_graphs::stitching::ForwardPartialPathStitcher;
use std::path::Path;
use std::path::PathBuf;
use tree_sitter_graph::Variables;

use crate::cli::util::ExistingPathBufValueParser;
use crate::cli::util::PathSpec;
use crate::loader::ContentProvider;
use crate::loader::FileReader;
use crate::loader::LanguageConfiguration;
use crate::loader::Loader;
use crate::test::Test;
use crate::test::TestResult;
use crate::CancellationFlag;
use crate::NoCancellation;

use super::util::iter_files_and_directories;
use super::util::ConsoleFileLogger;
use super::util::FileLogger;

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
    /// Test file or directory paths. Files or files inside directories ending in .skip are excluded.
    #[clap(
        value_name = "TEST_PATH",
        required = true,
        value_hint = ValueHint::AnyPath,
        value_parser = ExistingPathBufValueParser,
    )]
    pub test_paths: Vec<PathBuf>,

    /// Hide passing tests in output.
    #[clap(long, short = 'q')]
    pub quiet: bool,

    /// Hide failure error details.
    #[clap(long)]
    pub hide_error_details: bool,

    /// Show skipped files in output.
    #[clap(long)]
    pub show_skipped: bool,

    /// Save graph for tests matching output mode.
    /// Takes an optional path specification argument for the output file.
    /// [default: %n.graph.json]
    #[clap(
        long,
        short = 'G',
        value_name = "PATH_SPEC",
        num_args = 0..=1,
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
        num_args = 0..=1,
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
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "%n.html"
    )]
    pub save_visualization: Option<PathSpec>,

    /// Controls when graphs, paths, or visualization are saved.
    #[clap(
        long,
        value_enum,
        default_value_t = OutputMode::OnFailure,
    )]
    pub output_mode: OutputMode,

    /// Do not load builtins for tests.
    #[clap(long)]
    pub no_builtins: bool,
}

/// Flag to control output
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
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

impl TestArgs {
    pub fn new(test_paths: Vec<PathBuf>) -> Self {
        Self {
            test_paths,
            quiet: false,
            hide_error_details: false,
            show_skipped: false,
            save_graph: None,
            save_paths: None,
            save_visualization: None,
            output_mode: OutputMode::OnFailure,
            no_builtins: false,
        }
    }

    pub fn run(self, mut loader: Loader) -> anyhow::Result<()> {
        let mut total_result = TestResult::new();
        for (test_root, test_path, _) in iter_files_and_directories(self.test_paths.clone()) {
            let test_result = self.run_test(&test_root, &test_path, &mut loader)?;
            total_result.absorb(test_result);
        }
        if total_result.failure_count() > 0 {
            return Err(anyhow!(total_result.to_string()));
        }
        Ok(())
    }

    /// Run test file. Takes care of the output when an error is returned.
    fn run_test(
        &self,
        test_root: &Path,
        test_path: &Path,
        loader: &mut Loader,
    ) -> anyhow::Result<TestResult> {
        let mut file_status =
            ConsoleFileLogger::new(test_path, !self.quiet, !self.hide_error_details);
        match self.run_test_inner(test_root, test_path, loader, &mut file_status) {
            ok @ Ok(_) => ok,
            err @ Err(_) => {
                file_status.default_failure("error", None);
                err
            }
        }
    }

    fn run_test_inner(
        &self,
        test_root: &Path,
        test_path: &Path,
        loader: &mut Loader,
        file_status: &mut ConsoleFileLogger,
    ) -> anyhow::Result<TestResult> {
        let cancellation_flag = &NoCancellation;

        // If the file is skipped (ending in .skip) we construct the non-skipped path to see if we would support it.
        let load_path = if test_path.extension().map_or(false, |e| e == "skip") {
            test_path.with_extension("")
        } else {
            test_path.to_path_buf()
        };
        let mut file_reader = MappingFileReader::new(&load_path, test_path);
        let lc = match loader.load_for_file(&load_path, &mut file_reader, cancellation_flag)? {
            Some(sgl) => sgl,
            None => return Ok(TestResult::new()),
        };

        if test_path.components().any(|c| match c {
            std::path::Component::Normal(name) => (name.as_ref() as &Path)
                .extension()
                .map_or(false, |e| e == "skip"),
            _ => false,
        }) {
            if self.show_skipped {
                file_status.warning("skipped", None);
            }
            return Ok(TestResult::new());
        }

        file_status.processing();

        let source = file_reader.get(test_path)?;
        let default_fragment_path = test_path.strip_prefix(test_root).unwrap();
        let mut test = Test::from_source(test_path, source, default_fragment_path)?;
        if !self.no_builtins {
            self.load_builtins_into(&lc, &mut test.graph)?;
        }
        let mut globals = Variables::new();
        for test_fragment in &test.fragments {
            let result = if let Some(fa) = test_fragment
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
                    cancellation_flag,
                )
            } else if lc.matches_file(
                &test_fragment.path,
                &mut Some(test_fragment.source.as_ref()),
            )? {
                globals.clear();
                test_fragment.add_globals_to(&mut globals);
                lc.sgl.build_stack_graph_into(
                    &mut test.graph,
                    test_fragment.file,
                    &test_fragment.source,
                    &globals,
                    cancellation_flag,
                )
            } else {
                return Err(anyhow!(
                    "Test fragment {} not supported by language of test file {}",
                    test_fragment.path.display(),
                    test.path.display()
                ));
            };
            match result {
                Err(err) => {
                    file_status.failure(
                        "failed to build stack graph",
                        Some(&err.display_pretty(
                            test_path,
                            source,
                            lc.sgl.tsg_path(),
                            lc.sgl.tsg_source(),
                        )),
                    );
                    return Err(anyhow!("Failed to build graph for {}", test_path.display()));
                }
                Ok(_) => {}
            }
        }
        let mut partials = PartialPaths::new();
        let mut db = Database::new();
        for file in test.graph.iter_files() {
            partials.find_minimal_partial_path_set_in_file(
                &test.graph,
                file,
                &(cancellation_flag as &dyn CancellationFlag),
                |g, ps, p| {
                    db.add_partial_path(g, ps, p);
                },
            )?;
        }
        let result = test.run(&mut partials, &mut db, cancellation_flag)?;
        let success = self.handle_result(&result, file_status)?;
        if self.output_mode.test(!success) {
            let files = test.fragments.iter().map(|f| f.file).collect::<Vec<_>>();
            self.save_output(
                test_root,
                test_path,
                &test.graph,
                &mut partials,
                &mut db,
                &|_: &StackGraph, h: &Handle<File>| files.contains(h),
                success,
                cancellation_flag,
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

    fn handle_result(
        &self,
        result: &TestResult,
        file_status: &mut ConsoleFileLogger,
    ) -> anyhow::Result<bool> {
        let success = result.failure_count() == 0;
        if success {
            file_status.success("success", None);
        } else {
            file_status.failure(
                &format!(
                    "{}/{} assertions failed",
                    result.failure_count(),
                    result.count(),
                ),
                Some(&result.failures_iter().join("\n")),
            );
        }
        Ok(success)
    }

    fn save_output(
        &self,
        test_root: &Path,
        test_path: &Path,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &mut Database,
        filter: &dyn Filter,
        success: bool,
        cancellation_flag: &dyn CancellationFlag,
    ) -> anyhow::Result<()> {
        let save_graph = self
            .save_graph
            .as_ref()
            .map(|spec| spec.format(test_root, test_path));
        let save_paths = self
            .save_paths
            .as_ref()
            .map(|spec| spec.format(test_root, test_path));
        let save_visualization = self
            .save_visualization
            .as_ref()
            .map(|spec| spec.format(test_root, test_path));

        if let Some(path) = save_graph {
            self.save_graph(&path, &graph, filter)?;
            if !success || !self.quiet {
                println!("{}: graph at {}", test_path.display(), path.display());
            }
        }

        let mut db = if save_paths.is_some() || save_visualization.is_some() {
            self.compute_paths(graph, partials, db, filter, cancellation_flag)?
        } else {
            Database::new()
        };

        if let Some(path) = save_paths {
            self.save_paths(&path, graph, partials, &mut db, filter)?;
            if !success || !self.quiet {
                println!("{}: paths at {}", test_path.display(), path.display());
            }
        }

        if let Some(path) = save_visualization {
            self.save_visualization(&path, graph, partials, &mut db, filter, &test_path)?;
            if !success || !self.quiet {
                println!(
                    "{}: visualization at {}",
                    test_path.display(),
                    path.display()
                );
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
        let json = serde_json::to_string_pretty(&graph.to_serializable_filter(filter))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, json)?;
        Ok(())
    }

    fn compute_paths(
        &self,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &mut Database,
        filter: &dyn Filter,
        cancellation_flag: &dyn CancellationFlag,
    ) -> anyhow::Result<Database> {
        let references = graph
            .iter_nodes()
            .filter(|n| filter.include_node(graph, n))
            .collect::<Vec<_>>();
        let mut paths = Vec::new();
        ForwardPartialPathStitcher::find_all_complete_partial_paths(
            graph,
            partials,
            db,
            references.clone(),
            &cancellation_flag,
            |_, _, p| {
                paths.push(p.clone());
            },
        )?;
        let mut db = Database::new();
        for path in paths {
            db.add_partial_path(graph, partials, path);
        }
        Ok(db)
    }

    fn save_paths(
        &self,
        path: &Path,
        graph: &StackGraph,
        partials: &mut PartialPaths,
        db: &mut Database,
        filter: &dyn Filter,
    ) -> anyhow::Result<()> {
        let json =
            serde_json::to_string_pretty(&db.to_serializable_filter(graph, partials, filter))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, json)?;
        Ok(())
    }

    fn save_visualization(
        &self,
        path: &Path,
        graph: &StackGraph,
        paths: &mut PartialPaths,
        db: &mut Database,
        filter: &dyn Filter,
        test_path: &Path,
    ) -> anyhow::Result<()> {
        let html = graph.to_html_string(&format!("{}", test_path.display()), paths, db, filter)?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, html)?;
        Ok(())
    }
}

struct MappingFileReader<'a> {
    inner: FileReader,
    instead_of: &'a Path,
    load: &'a Path,
}

impl<'a> MappingFileReader<'a> {
    fn new(instead_of: &'a Path, load: &'a Path) -> Self {
        Self {
            inner: FileReader::new(),
            instead_of,
            load,
        }
    }

    fn get(&mut self, path: &Path) -> std::io::Result<&str> {
        let path = if path == self.instead_of {
            self.load
        } else {
            path
        };
        self.inner.get(path)
    }
}

impl ContentProvider for MappingFileReader<'_> {
    fn get(&mut self, path: &Path) -> std::io::Result<Option<&str>> {
        self.get(path).map(Some)
    }
}
