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
            path_fn(|p| normalize(p).map(|p| p.as_os_str().to_os_string())),
        );
        functions.add("path-split".into(), PathSplit);
    }

    pub fn path_fn<F>(f: F) -> impl Function
    where
        F: Fn(&Path) -> Option<std::ffi::OsString>,
    {
        PathFn(f)
    }

    struct PathFn<F>(F)
    where
        F: Fn(&Path) -> Option<std::ffi::OsString>;

    impl<F> Function for PathFn<F>
    where
        F: Fn(&Path) -> Option<std::ffi::OsString>,
    {
        fn call(
            &self,
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
            &self,
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
            &self,
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

    /// Normalize a path, removing things like `.` and `..` wherever possible.
    // Based on the following code from Cargo:
    // https://github.com/rust-lang/cargo/blob/e515c3277bf0681bfc79a9e763861bfe26bb05db/crates/cargo-util/src/paths.rs#L73-L106
    // Licensed under MIT license & Apache License (Version 2.0)
    pub fn normalize(path: &Path) -> Option<PathBuf> {
        let mut components = path.components().peekable();
        let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
            components.next();
            PathBuf::from(c.as_os_str())
        } else {
            PathBuf::new()
        };

        let mut has_root = false;
        let mut normal_components = 0usize;
        for component in components {
            match component {
                Component::Prefix(..) => unreachable!(),
                Component::RootDir => {
                    has_root = true;
                    ret.push(component.as_os_str());
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    if normal_components > 0 {
                        normal_components -= 1;
                        ret.pop();
                    } else if has_root {
                        return None;
                    } else {
                        ret.push(component.as_os_str());
                    }
                }
                Component::Normal(c) => {
                    normal_components += 1;
                    ret.push(c);
                }
            }
        }
        Some(ret)
    }
}
