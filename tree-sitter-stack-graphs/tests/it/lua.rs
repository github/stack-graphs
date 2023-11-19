// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use lua_helpers::new_lua;
use stack_graphs::graph::StackGraph;
use tree_sitter_stack_graphs::lua::StackGraphLanguageLua;
use tree_sitter_stack_graphs::NoCancellation;

trait CheckLua {
    fn check(&self, graph: &mut StackGraph, chunk: &str) -> Result<(), mlua::Error>;
}

impl CheckLua for mlua::Lua {
    fn check(&self, graph: &mut StackGraph, chunk: &str) -> Result<(), mlua::Error> {
        self.scope(|scope| {
            let graph = graph.lua_ref_mut(&scope)?;
            self.load(chunk).set_name("test chunk").call(graph)
        })
    }
}

// This doesn't build a very _interesting_ stack graph, but it does test that the end-to-end
// spackle all works correctly.
#[test]
fn can_build_stack_graph_from_lua() -> Result<(), anyhow::Error> {
    const LUA: &[u8] = br#"
      function process(parsed, file)
        -- TODO: fill in the definiens span from the parse tree root
        local module = file:internal_scope_node()
        module:add_edge_from(file:root_node())
      end
    "#;

    let code = r#"
      def double(x):
          return x * 2
    "#;
    let mut graph = StackGraph::new();
    let file = graph.get_or_create_file("test.py");
    let language =
        StackGraphLanguageLua::from_static_str(tree_sitter_python::language(), LUA, "test");
    language.build_stack_graph_into(&mut graph, file, code, &NoCancellation)?;

    let l = new_lua()?;
    l.check(
        &mut graph,
        r#"
          local graph = ...
          local file = graph:file("test.py")
          assert_deepeq("nodes", {
            "[test.py(0) scope]",
          }, iter_tostring(file:nodes()))
          assert_deepeq("edges", {
            "[root] -0-> [test.py(0) scope]",
          }, iter_tostring(values(file:edges())))
        "#,
    )?;

    Ok(())
}
