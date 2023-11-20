// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use lua_helpers::new_lua;
use stack_graphs::graph::NodeID;
use stack_graphs::graph::StackGraph;

trait CheckLua {
    fn check(&self, graph: &mut StackGraph, chunk: &str) -> Result<(), mlua::Error>;
}

impl CheckLua for mlua::Lua {
    fn check(&self, graph: &mut StackGraph, chunk: &str) -> Result<(), mlua::Error> {
        self.scope(|scope| {
            let graph = scope.create_userdata_ref_mut(graph);
            self.load(chunk).set_name("test chunk").call(graph)
        })
    }
}

#[test]
fn can_create_nodes_from_lua() -> Result<(), anyhow::Error> {
    let l = new_lua()?;
    let mut graph = StackGraph::new();
    l.check(
        &mut graph,
        r#"
              local graph = ...
              local file = graph:file("test.py")
              local n0 = file:internal_scope_node()
              local n1 = file:internal_scope_node()
              assert_eq("local ID", 0, n0:local_id())
              assert_eq("local ID", 1, n1:local_id())
            "#,
    )?;

    let node_count = graph.iter_nodes().count();
    assert_eq!(node_count, 4); // Include the predefined ROOT and JUMP TO nodes in the count

    let file = graph.get_file("test.py").expect("Cannot find file");
    let n0 = graph.node_for_id(NodeID::new_in_file(file, 0));
    assert!(n0.is_some(), "Cannot find node 0");
    let n1 = graph.node_for_id(NodeID::new_in_file(file, 1));
    assert!(n1.is_some(), "Cannot find node 1");

    Ok(())
}

#[test]
fn can_set_source_info_from_lua() -> Result<(), anyhow::Error> {
    let l = new_lua()?;
    let mut graph = StackGraph::new();
    l.check(
        &mut graph,
        r#"
          local graph = ...
          local file = graph:file("test.py")
          local n0 = file:internal_scope_node()

          n0:set_syntax_type("function")
          assert_eq("syntax type", "function", n0:syntax_type())

          n0:set_span {
            start={line=1, column={utf8_offset=1}},
            ["end"]={line=1, column={utf8_offset=19}},
          }
          assert_eq("start line", 1, n0:span().start.line)
          assert_eq("start column", 1, n0:span().start.column.utf8_offset)
          assert_eq("end line", 1, n0:span()["end"].line)
          assert_eq("end column", 19, n0:span()["end"].column.utf8_offset)

          n0:set_definiens_span {
            start={line=2, column={utf8_offset=1}},
            ["end"]={line=78, column={utf8_offset=24}},
          }
          assert_eq("start line", 2, n0:definiens_span().start.line)
          assert_eq("start column", 1, n0:definiens_span().start.column.utf8_offset)
          assert_eq("end line", 78, n0:definiens_span()["end"].line)
          assert_eq("end column", 24, n0:definiens_span()["end"].column.utf8_offset)

          assert_eq("node", "[test.py(0) scope (function) at 1:1-1:19 def 2:1-78:24]", tostring(n0))
        "#,
    )?;
    Ok(())
}

#[test]
fn can_set_debug_info_from_lua() -> Result<(), anyhow::Error> {
    let l = new_lua()?;
    let mut graph = StackGraph::new();
    l.check(
        &mut graph,
        r#"
          local graph = ...
          local file = graph:file("test.py")
          local n0 = file:internal_scope_node()
          n0:set_debug_info("k1", "v1")
          n0:set_debug_info("k2", "v2")
          local expected = { k1="v1", k2="v2" }
          assert_deepeq("debug info", expected, n0:debug_info())
        "#,
    )?;
    Ok(())
}

#[test]
fn can_create_edges_from_lua() -> Result<(), anyhow::Error> {
    let l = new_lua()?;
    let mut graph = StackGraph::new();
    l.check(
        &mut graph,
        r#"
          local graph = ...
          local root = graph:root_node()
          local file = graph:file("test.py")
          local n0 = file:internal_scope_node()
          local n1 = file:internal_scope_node()
          local e0 = n0:add_edge_to(n1)
          local e1 = n0:add_edge_from(n1, 10)
          local e2 = n0:add_edge_to(root)
          local e3 = n0:add_edge_from(root)
          assert_eq("edge", "[test.py(0) scope] -0-> [test.py(1) scope]", tostring(e0))
          assert_eq("edge", "[test.py(1) scope] -10-> [test.py(0) scope]", tostring(e1))

          assert_deepeq("node edges", {
            "[test.py(0) scope] -0-> [root]",
            "[test.py(0) scope] -0-> [test.py(1) scope]",
          }, iter_tostring(values(n0:outgoing_edges())))
          assert_deepeq("node edges", {
            "[test.py(1) scope] -10-> [test.py(0) scope]",
          }, iter_tostring(values(n1:outgoing_edges())))
          assert_deepeq("node edges", {
            "[root] -0-> [test.py(0) scope]",
          }, iter_tostring(values(root:outgoing_edges())))

          assert_deepeq("file edges", {
            "[root] -0-> [test.py(0) scope]",
            "[test.py(0) scope] -0-> [root]",
            "[test.py(0) scope] -0-> [test.py(1) scope]",
            "[test.py(1) scope] -10-> [test.py(0) scope]",
          }, iter_tostring(values(file:edges())))

          assert_deepeq("graph edges", {
            "[root] -0-> [test.py(0) scope]",
            "[test.py(0) scope] -0-> [root]",
            "[test.py(0) scope] -0-> [test.py(1) scope]",
            "[test.py(1) scope] -10-> [test.py(0) scope]",
          }, iter_tostring(values(graph:edges())))
        "#,
    )?;
    Ok(())
}

#[test]
fn can_create_all_node_types_from_lua() -> Result<(), anyhow::Error> {
    let l = new_lua()?;
    let mut graph = StackGraph::new();
    l.check(
        &mut graph,
        r#"
              local graph = ...
              local root = graph:root_node()
              local jump_to = graph:jump_to_node()
              local file = graph:file("test.py")
              local file_root = file:root_node()
              local file_jump_to = file:jump_to_node()
              local drop_scopes = file:drop_scopes_node()
              local exported = file:exported_scope_node()
              local internal = file:internal_scope_node()
              local pop_scoped_symbol = file:pop_scoped_symbol_node("foo")
              local scoped_definition = file:scoped_definition_node("bar")
              local pop_symbol = file:pop_symbol_node("foo")
              local definition = file:definition_node("bar")
              local push_scoped_symbol = file:push_scoped_symbol_node("foo", exported)
              local scoped_reference = file:scoped_reference_node("bar", exported)
              local push_symbol = file:push_symbol_node("foo")
              local reference = file:reference_node("bar")

              assert_deepeq("nodes", {
                "[root]",
                "[jump to scope]",
                "[test.py(0) drop scopes]",
                "[test.py(1) exported scope]",
                "[test.py(2) scope]",
                "[test.py(3) pop scoped foo]",
                "[test.py(4) scoped definition bar]",
                "[test.py(5) pop foo]",
                "[test.py(6) definition bar]",
                "[test.py(7) push scoped foo test.py(1)]",
                "[test.py(8) scoped reference bar test.py(1)]",
                "[test.py(9) push foo]",
                "[test.py(10) reference bar]",
              }, iter_tostring(graph:nodes()))
            "#,
    )?;
    Ok(())
}

#[test]
fn can_iterate_nodes_in_file() -> Result<(), anyhow::Error> {
    let l = new_lua()?;
    let mut graph = StackGraph::new();
    l.check(
        &mut graph,
        r#"
              local graph = ...
              local file1 = graph:file("test1.py")
              local file2 = graph:file("test2.py")
              file1:internal_scope_node()
              file2:internal_scope_node()
              file1:internal_scope_node()
              file2:internal_scope_node()
              file2:internal_scope_node()
              file1:internal_scope_node()
              file2:internal_scope_node()
              file1:internal_scope_node()

              assert_deepeq("nodes", {
                "[test1.py(0) scope]",
                "[test1.py(1) scope]",
                "[test1.py(2) scope]",
                "[test1.py(3) scope]",
              }, iter_tostring(file1:nodes()))

              assert_deepeq("nodes", {
                "[test2.py(0) scope]",
                "[test2.py(1) scope]",
                "[test2.py(2) scope]",
                "[test2.py(3) scope]",
              }, iter_tostring(file2:nodes()))
            "#,
    )?;
    Ok(())
}
