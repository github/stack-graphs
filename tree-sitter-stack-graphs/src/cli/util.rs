// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use anyhow::anyhow;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use tree_sitter_graph::parse_error::TreeWithParseErrorVec;

use crate::cli::MAX_PARSE_ERRORS;

pub fn path_exists(path: &OsStr) -> anyhow::Result<PathBuf> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(anyhow!("path does not exist"));
    }
    Ok(path)
}

/// A path specification that can be formatted into a path based on a root and path
/// contained in that root.
pub struct PathSpec {
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
        match crate::functions::path::normalize(&path) {
            Some(path) => path,
            None => panic!("Cannot normalize '{}'", path.display()),
        }
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

pub fn map_parse_errors(
    test_path: &Path,
    parse_errors: &TreeWithParseErrorVec,
    source: &str,
    prefix: &str,
) -> anyhow::Error {
    let mut error = String::new();
    let parse_errors = parse_errors.errors();
    for parse_error in parse_errors.iter().take(MAX_PARSE_ERRORS) {
        let line = parse_error.node().start_position().row;
        let column = parse_error.node().start_position().column;
        error.push_str(&format!(
            "{}{}:{}:{}: {}\n",
            prefix,
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

pub fn duration_from_seconds_str(s: &str) -> Result<Duration, anyhow::Error> {
    Ok(Duration::new(s.parse()?, 0))
}
