// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Construct stack graphs using a Lua script that consumes a tree-sitter parse tree

use std::borrow::Cow;

use lsp_positions::lua::Module as _;
use mlua::Lua;
use mlua_tree_sitter::Module as _;
use mlua_tree_sitter::WithSource;
use stack_graphs::arena::Handle;
use stack_graphs::graph::File;
use stack_graphs::graph::StackGraph;

use crate::parse_file;
use crate::BuildError;
use crate::CancellationFlag;

/// Holds information about how to construct stack graphs for a particular language.
pub struct StackGraphLanguageLua {
    language: tree_sitter::Language,
    lua_source: Cow<'static, [u8]>,
    lua_source_name: String,
}

impl StackGraphLanguageLua {
    /// Creates a new stack graph language for the given language, loading the Lua stack graph
    /// construction rules from a static string.
    pub fn from_static_str(
        language: tree_sitter::Language,
        lua_source: &'static [u8],
        lua_source_name: &str,
    ) -> StackGraphLanguageLua {
        StackGraphLanguageLua {
            language,
            lua_source: Cow::from(lua_source),
            lua_source_name: lua_source_name.to_string(),
        }
    }

    /// Creates a new stack graph language for the given language, loading the Lua stack graph
    /// construction rules from a string.
    pub fn from_str(
        language: tree_sitter::Language,
        lua_source: &[u8],
        lua_source_name: &str,
    ) -> StackGraphLanguageLua {
        StackGraphLanguageLua {
            language,
            lua_source: Cow::from(lua_source.to_vec()),
            lua_source_name: lua_source_name.to_string(),
        }
    }

    pub fn language(&self) -> tree_sitter::Language {
        self.language
    }

    pub fn lua_source_name(&self) -> &str {
        &self.lua_source_name
    }

    pub fn lua_source(&self) -> &Cow<'static, [u8]> {
        &self.lua_source
    }

    /// Executes the graph construction rules for this language against a source file, creating new
    /// nodes and edges in `stack_graph`.  Any new nodes that we create will belong to `file`.
    /// (The source file must be implemented in this language, otherwise you'll probably get a
    /// parse error.)
    pub fn build_stack_graph_into<'a>(
        &'a self,
        stack_graph: &'a mut StackGraph,
        file: Handle<File>,
        source: &'a str,
        cancellation_flag: &'a dyn CancellationFlag,
    ) -> Result<(), BuildError> {
        // Create a Lua environment and load the language's stack graph rules.
        // TODO: Sandbox the Lua environment
        let lua = Lua::new();
        lua.open_lsp_positions()?;
        lua.open_ltreesitter()?;
        lua.load(self.lua_source.as_ref())
            .set_name(&self.lua_source_name)
            .exec()?;
        let process: mlua::Function = lua.globals().get("process")?;

        // Parse the source using the requested grammar.
        let tree = parse_file(self.language, source, cancellation_flag)?;
        let tree = tree.with_source(source.as_bytes());

        // Invoke the Lua `process` function with the parsed tree and the stack graph file.
        // TODO: Add a debug hook that checks the cancellation flag during execution
        lua.scope(|scope| {
            let file = stack_graph.file_lua_ref_mut(file, scope)?;
            process.call((tree, file))
        })?;
        Ok(())
    }
}
