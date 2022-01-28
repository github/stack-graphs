// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use tree_sitter_graph::functions::Functions;

pub fn add_path_functions(functions: &mut Functions) {
    functions.add("normalize-path".into(), path::NormalizePath);
    functions.add("resolve-path".into(), path::ResolvePath);
}

pub mod path {
    use std::path::Component;
    use std::path::Path;
    use std::path::PathBuf;
    use tree_sitter_graph::functions::Function;
    use tree_sitter_graph::functions::Parameters;
    use tree_sitter_graph::graph::Graph;
    use tree_sitter_graph::graph::Value;
    use tree_sitter_graph::ExecutionError;

    pub struct NormalizePath;

    impl Function for NormalizePath {
        fn call(
            &mut self,
            _graph: &mut Graph,
            _source: &str,
            parameters: &mut dyn Parameters,
        ) -> Result<Value, ExecutionError> {
            let path = parameters.param()?.into_string()?;
            parameters.finish()?;

            let path = Path::new(&path);
            let path = normalize_path(&path.to_path_buf());

            Ok(path.to_str().unwrap().into())
        }
    }

    pub struct ResolvePath;

    impl Function for ResolvePath {
        fn call(
            &mut self,
            _graph: &mut Graph,
            _source: &str,
            parameters: &mut dyn Parameters,
        ) -> Result<Value, ExecutionError> {
            let base_path = parameters.param()?.into_string()?;
            let path = parameters.param()?.into_string()?;
            parameters.finish()?;

            // FIXME .parent() assumes this is a file path, this API needs some thought
            let path = Path::new(&base_path).parent().unwrap().join(path);

            Ok(path.to_str().unwrap().into())
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
    pub fn normalize_path(path: &PathBuf) -> PathBuf {
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
