// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use anyhow::Context as _;
use clap::ArgEnum;
use clap::ValueHint;
use colored::Colorize as _;
use stack_graphs::graph::StackGraph;
use stack_graphs::paths::Paths;
use std::ffi::OsStr;
use std::ffi::OsString;
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
pub struct Command {
    #[clap(flatten)]
    loader: LoaderArgs,

    /// Test file or directory paths.
    #[clap(value_name = "TEST_PATH", required = true, value_hint = ValueHint::AnyPath, parse(from_os_str), validator_os = path_exists)]
    tests: Vec<PathBuf>,

    /// Hide passing tests.
    #[clap(long)]
    hide_passing: bool,

    /// Hide failure error details.
    #[clap(long)]
    hide_failure_errors: bool,

    /// Show ignored files in output.
    #[clap(long)]
    show_ignored: bool,

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
    save_graph: Option<PathSpec>,

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
    save_paths: Option<PathSpec>,

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
    save_visualization: Option<PathSpec>,

    /// Controls when graphs, paths, or visualization are saved.
    #[clap(long, arg_enum, default_value_t = OutputMode::OnFailure)]
    output_mode: OutputMode,
}

fn path_exists(path: &OsStr) -> anyhow::Result<PathBuf> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(anyhow!("path does not exist"));
    }
    Ok(path)
}

impl Command {
    pub fn run(&self) -> anyhow::Result<()> {
        let mut loader = self.loader.new_loader()?;
        let mut total_failure_count = 0;
        for test_path in &self.tests {
            if test_path.is_dir() {
                let test_root = test_path;
                for test_entry in WalkDir::new(test_path)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let test_path = test_entry.path();
                    total_failure_count +=
                        self.run_test_with_context(test_root, test_path, &mut loader)?;
                }
            } else {
                let test_root = test_path.parent().unwrap();
                total_failure_count +=
                    self.run_test_with_context(test_root, test_path, &mut loader)?;
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

    /// Run test file and add error context to any failures that are returned.
    fn run_test_with_context(
        &self,
        test_root: &Path,
        test_path: &Path,
        loader: &mut Loader,
    ) -> anyhow::Result<usize> {
        self.run_test(test_root, test_path, loader)
            .with_context(|| format!("Error running test {}", test_path.display()))
    }

    /// Run test file.
    fn run_test(
        &self,
        test_root: &Path,
        test_path: &Path,
        loader: &mut Loader,
    ) -> anyhow::Result<usize> {
        let source = String::from_utf8(std::fs::read(test_path)?)?;
        let sgl = match loader.load_for_file(test_path, Some(&source))? {
            Some(sgl) => sgl,
            None => {
                if self.show_ignored {
                    println!("{} {}", "⦵".dimmed(), test_path.display());
                }
                return Ok(0);
            }
        };
        let default_fragment_path = test_path.strip_prefix(test_root).unwrap();
        let mut test = Test::from_source(&test_path, &source, default_fragment_path)?;
        self.load_builtins_into(sgl, &mut test.graph)
            .with_context(|| format!("Loading builtins into {}", test_path.display()))?;
        for test_fragment in &test.fragments {
            let fragment_path = Path::new(test.graph[test_fragment.file].name()).to_path_buf();
            if test_path.extension() != fragment_path.extension() {
                return Err(anyhow!(
                    "Test fragment {} has different file extension than test file {}",
                    fragment_path.display(),
                    test_path.display()
                ));
            }
            self.build_fragment_stack_graph_into(
                &fragment_path,
                sgl,
                test_fragment,
                &mut test.graph,
            )?;
        }
        let result = test.run();
        let success = self.handle_result(test_path, &result)?;
        if self.output_mode.test(!success) {
            self.save_output(test_root, test_path, &test.graph, &mut test.paths, success)?;
        }
        Ok(result.failure_count())
    }

    fn load_builtins_into(
        &self,
        sgl: &mut StackGraphLanguage,
        graph: &mut StackGraph,
    ) -> anyhow::Result<()> {
        if let Err(h) = graph.add_graph(sgl.builtins()) {
            return Err(anyhow!("Duplicate builtin file {}", &graph[h]));
        }
        Ok(())
    }

    fn build_fragment_stack_graph_into(
        &self,
        test_path: &Path,
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
                Err(self.map_parse_errors(test_path, &parse_errors, &test_fragment.source))
            }
            Err(e) => Err(e.into()),
            Ok(_) => Ok(()),
        }
    }

    fn map_parse_errors(
        &self,
        test_path: &Path,
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
                test_path.display(),
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
        success: bool,
    ) -> anyhow::Result<()> {
        if let Some(path) = self
            .save_graph
            .as_ref()
            .map(|spec| spec.format(test_root, test_path))
        {
            self.save_graph(&path, &graph)?;
            if !success || !self.hide_passing {
                println!("  Graph: {}", path.display());
            }
        }
        if let Some(path) = self
            .save_paths
            .as_ref()
            .map(|spec| spec.format(test_root, test_path))
        {
            self.save_paths(&path, paths, graph)?;
            if !success || !self.hide_passing {
                println!("  Paths: {}", path.display());
            }
        }
        if let Some(path) = self
            .save_visualization
            .as_ref()
            .map(|spec| spec.format(test_root, test_path))
        {
            self.save_visualization(&path, paths, graph, &test_path)?;
            if !success || !self.hide_passing {
                println!("  Visualization: {}", path.display());
            }
        }
        Ok(())
    }

    fn save_graph(&self, path: &Path, graph: &StackGraph) -> anyhow::Result<()> {
        let json = graph.to_json().to_string_pretty()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, json)
            .with_context(|| format!("Unable to write graph {}", path.display()))?;
        Ok(())
    }

    fn save_paths(&self, path: &Path, paths: &mut Paths, graph: &StackGraph) -> anyhow::Result<()> {
        let json = paths.to_json(graph, |_, _, _| true).to_string_pretty()?;
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
        test_path: &Path,
    ) -> anyhow::Result<()> {
        let html = graph.to_html_string(paths, &format!("{}", test_path.display()))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, html)
            .with_context(|| format!("Unable to write graph {}", path.display()))?;
        Ok(())
    }
}

/// A path specification that can be formatted into a path based on a root and path
/// contained in that root.
struct PathSpec {
    spec: String,
}

impl PathSpec {
    pub fn format(&self, root: &Path, full_path: &Path) -> PathBuf {
        if !full_path.starts_with(root) {
            panic!(
                "Path {} not contained in root {}",
                full_path.display(),
                root.display()
            );
        }
        let relative_path = full_path.strip_prefix(root).unwrap();
        if relative_path.is_absolute() {
            panic!(
                "Path {} not relative to root {}",
                full_path.display(),
                root.display()
            );
        }
        self.format_path(
            &self.dir_os_str(Some(root)),
            &self.dir_os_str(relative_path.parent()),
            relative_path.file_stem(),
            relative_path.extension(),
        )
    }

    /// Convert an optional directory path to an OsString representation. If the
    /// path is missing or empty, we return `.`.
    fn dir_os_str(&self, path: Option<&Path>) -> OsString {
        let s = path.map_or("".into(), |p| p.as_os_str().to_os_string());
        let s = if s.is_empty() { ".".into() } else { s };
        s
    }

    fn format_path(
        &self,
        root: &OsStr,
        dirs: &OsStr,
        name: Option<&OsStr>,
        ext: Option<&OsStr>,
    ) -> PathBuf {
        let mut path = OsString::new();
        let mut in_placeholder = false;
        for c in self.spec.chars() {
            if in_placeholder {
                in_placeholder = false;
                match c {
                    '%' => path.push("%"),
                    'd' => {
                        path.push(dirs);
                    }
                    'e' => {
                        if let Some(ext) = ext {
                            path.push(".");
                            path.push(ext);
                        }
                    }
                    'n' => {
                        if let Some(name) = name {
                            path.push(name);
                        }
                    }
                    'r' => path.push(root),
                    c => panic!("Unsupported placeholder '%{}'", c),
                }
            } else if c == '%' {
                in_placeholder = true;
            } else {
                path.push(c.to_string());
            }
        }
        if in_placeholder {
            panic!("Unsupported '%' at end");
        }
        let path = Path::new(&path);
        tree_sitter_stack_graphs::functions::path::normalize(&path)
    }
}

impl std::str::FromStr for PathSpec {
    type Err = clap::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { spec: s.into() })
    }
}

impl From<&str> for PathSpec {
    fn from(s: &str) -> Self {
        Self { spec: s.into() }
    }
}
