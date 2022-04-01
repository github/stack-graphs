// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Define tree-sitter-graph functions

use tree_sitter_graph::functions::Functions;

pub fn add_path_functions(functions: &mut Functions) {
    functions.add("path-dir".into(), path::PathDir);
    functions.add("path-fileext".into(), path::PathFileExt);
    functions.add("path-filename".into(), path::PathFileName);
    functions.add("path-filestem".into(), path::PathFileStem);
    functions.add("path-join".into(), path::PathJoin);
    functions.add("path-normalize".into(), path::PathNormalize);
    functions.add("path-split".into(), path::PathSplit);
}

pub mod path {
    use std::path::Component;
    use std::path::PathBuf;
    use tree_sitter_graph::functions::Function;
    use tree_sitter_graph::functions::Parameters;
    use tree_sitter_graph::graph::Graph;
    use tree_sitter_graph::graph::Value;
    use tree_sitter_graph::ExecutionError;

    pub struct PathDir;
    pub struct PathFileExt;
    pub struct PathFileName;
    pub struct PathFileStem;
    pub struct PathJoin;
    pub struct PathNormalize;
    pub struct PathSplit;

    impl Function for PathDir {
        fn call(
            &mut self,
            _graph: &mut Graph,
            _source: &str,
            parameters: &mut dyn Parameters,
        ) -> Result<Value, ExecutionError> {
            let path = PathBuf::from(parameters.param()?.into_string()?);
            parameters.finish()?;

            let path = path.parent();
            Ok(path
                .map(|p| p.to_str().unwrap().into())
                .unwrap_or(Value::Null))
        }
    }

    impl Function for PathFileExt {
        fn call(
            &mut self,
            _graph: &mut Graph,
            _source: &str,
            parameters: &mut dyn Parameters,
        ) -> Result<Value, ExecutionError> {
            let path = PathBuf::from(parameters.param()?.into_string()?);
            parameters.finish()?;

            let path = path.extension();
            Ok(path
                .map(|p| p.to_str().unwrap().into())
                .unwrap_or(Value::Null))
        }
    }

    impl Function for PathFileName {
        fn call(
            &mut self,
            _graph: &mut Graph,
            _source: &str,
            parameters: &mut dyn Parameters,
        ) -> Result<Value, ExecutionError> {
            let path = PathBuf::from(parameters.param()?.into_string()?);
            parameters.finish()?;

            let path = path.file_name();
            Ok(path
                .map(|p| p.to_str().unwrap().into())
                .unwrap_or(Value::Null))
        }
    }

    impl Function for PathFileStem {
        fn call(
            &mut self,
            _graph: &mut Graph,
            _source: &str,
            parameters: &mut dyn Parameters,
        ) -> Result<Value, ExecutionError> {
            let path = PathBuf::from(parameters.param()?.into_string()?);
            parameters.finish()?;

            let path = path.file_stem();
            Ok(path
                .map(|p| p.to_str().unwrap().into())
                .unwrap_or(Value::Null))
        }
    }

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

    impl Function for PathNormalize {
        fn call(
            &mut self,
            _graph: &mut Graph,
            _source: &str,
            parameters: &mut dyn Parameters,
        ) -> Result<Value, ExecutionError> {
            let path = PathBuf::from(parameters.param()?.into_string()?);
            parameters.finish()?;

            let path = normalize_path(&path);

            Ok(path.to_str().unwrap().into())
        }
    }

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
    fn normalize_path(path: &PathBuf) -> PathBuf {
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