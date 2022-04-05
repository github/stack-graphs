// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Define tree-sitter-graph functions

pub use path::add_path_functions;

pub mod path {
    use std::path::Component;
    use std::path::Path;
    use std::path::PathBuf;
    use tree_sitter_graph::functions::Function;
    use tree_sitter_graph::functions::Functions;
    use tree_sitter_graph::functions::Parameters;
    use tree_sitter_graph::graph::Graph;
    use tree_sitter_graph::graph::Value;
    use tree_sitter_graph::ExecutionError;

    pub fn add_path_functions(functions: &mut Functions) {
        functions.add(
            "path-dir".into(),
            path_fn(|p| p.parent().map(|s| s.as_os_str().to_os_string())),
        );
        functions.add(
            "path-fileext".into(),
            path_fn(|p| p.extension().map(|s| s.to_os_string())),
        );
        functions.add(
            "path-filename".into(),
            path_fn(|p| p.file_name().map(|s| s.to_os_string())),
        );
        functions.add(
            "path-filestem".into(),
            path_fn(|p| p.file_stem().map(|s| s.to_os_string())),
        );
        functions.add("path-join".into(), PathJoin);
        functions.add(
            "path-normalize".into(),
            path_fn(|p| Some(normalize_path(p).as_os_str().to_os_string())),
        );
        functions.add("path-split".into(), PathSplit);
    }

    pub fn path_fn<F>(f: F) -> impl Function
    where
        F: FnMut(&Path) -> Option<std::ffi::OsString>,
    {
        PathFn(f)
    }

    struct PathFn<F>(F)
    where
        F: FnMut(&Path) -> Option<std::ffi::OsString>;

    impl<F> Function for PathFn<F>
    where
        F: FnMut(&Path) -> Option<std::ffi::OsString>,
    {
        fn call(
            &mut self,
            _graph: &mut Graph,
            _source: &str,
            parameters: &mut dyn Parameters,
        ) -> Result<Value, ExecutionError> {
            let path = PathBuf::from(parameters.param()?.into_string()?);
            parameters.finish()?;

            let path = self.0(&path);
            Ok(path
                .map(|s| {
                    s.into_string()
                        .unwrap_or_else(|s| s.to_string_lossy().to_string())
                        .into()
                })
                .unwrap_or(Value::Null))
        }
    }

    struct PathJoin;

    impl Function for PathJoin {
        fn call(
            &mut self,
            _graph: &mut Graph,
            _source: &str,
            parameters: &mut dyn Parameters,
        ) -> Result<Value, ExecutionError> {
            let mut path = PathBuf::new();
            while let Ok(component) = parameters.param() {
                path = path.join(component.into_string()?);
            }

            Ok(path.to_str().unwrap().into())
        }
    }

    struct PathSplit;

    impl Function for PathSplit {
        fn call(
            &mut self,
            _graph: &mut Graph,
            _source: &str,
            parameters: &mut dyn Parameters,
        ) -> Result<Value, ExecutionError> {
            let path = PathBuf::from(parameters.param()?.into_string()?);
            parameters.finish()?;

            let components = path
                .components()
                .map(|c| c.as_os_str().to_str().unwrap().into())
                .collect::<Vec<_>>();
            Ok(components.into())
        }
    }

    /// Normalize a path, removing things like `.` and `..`.
    ///
    /// CAUTION: This does not resolve symlinks (unlike
    /// [`std::fs::canonicalize`]). This may cause incorrect or surprising
    /// behavior at times. This should be used carefully. Unfortunately,
    /// [`std::fs::canonicalize`] can be hard to use correctly, since it can often
    /// fail, or on Windows returns annoying device paths. This is a problem Cargo
    /// needs to improve on.
    // Copied from Cargo
    // https://github.com/rust-lang/cargo/blob/e515c3277bf0681bfc79a9e763861bfe26bb05db/crates/cargo-util/src/paths.rs#L73-L106
    // Licensed under MIT license & Apache License (Version 2.0)
    fn normalize_path(path: &Path) -> PathBuf {
        let mut components = path.components().peekable();
        let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
            components.next();
            PathBuf::from(c.as_os_str())
        } else {
            PathBuf::new()
        };

        for component in components {
            match component {
                Component::Prefix(..) => unreachable!(),
                Component::RootDir => {
                    ret.push(component.as_os_str());
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    ret.pop();
                }
                Component::Normal(c) => {
                    ret.push(c);
                }
            }
        }
        ret
    }
}
