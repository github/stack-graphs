// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::collections::HashSet;

use maplit::hashset;
use stack_graphs::graph::NodeID;
use stack_graphs::graph::StackGraph;

const TEST_PRELUDE: &str = r#"
  function assert_eq(thing, expected, actual)
    if expected ~= actual then
      error("Expected "..thing.." "..expected..", got "..actual)
    end
  end

  function deepeq(t1, t2, prefix)
    prefix = prefix or ""
    local ty1 = type(t1)
    local ty2 = type(t2)
    if ty1 ~= ty2 then
      local msg = "different types for lhs"..prefix.." ("..ty1..") and rhs"..prefix.." ("..ty2..")"
      return false, {msg}
    end

    -- non-table types can be directly compared
    if ty1 ~= 'table' and ty2 ~= 'table' then
      if t1 ~= t2 then
        local msg = "different values for lhs"..prefix.." ("..t1..") and rhs"..prefix.." ("..t2..")"
        return false, {msg}
      end
      return true, {}
    end

    equal = true
    diffs = {}
    for k2, v2 in pairs(t2) do
      local v1 = t1[k2]
      if v1 == nil then
        equal = false
        diffs[#diffs+1] = "missing lhs"..prefix.."."..k2
      else
        local e, d = deepeq(v1, v2, prefix.."."..k2)
        equal = equal and e
        table.move(d, 1, #d, #diffs+1, diffs)
      end
    end
    for k1, v1 in pairs(t1) do
      local v2 = t2[k1]
      if v2 == nil then
        equal = false
        diffs[#diffs+1] = "missing rhs"..prefix.."."..k1
      end
    end
    return equal, diffs
  end

  function assert_deepeq(thing, expected, actual)
    local eq, diffs = deepeq(expected, actual)
    if not eq then
      error("Unexpected "..thing..": "..table.concat(diffs, ", "))
    end
  end
"#;

fn new_lua() -> mlua::Lua {
    let l = mlua::Lua::new();
    l.load(TEST_PRELUDE)
        .set_name("test prelude")
        .exec()
        .expect("Error loading test prelude");
    l
}

trait CheckLua {
    /// Executes a chunk of Lua code.  If it returns a string, interprets that string as an
    /// error message, and translates that into an `anyhow` error.
    fn check_without_graph(&self, chunk: &str) -> Result<(), mlua::Error>;
    fn check(&self, graph: &mut StackGraph, chunk: &str) -> Result<(), mlua::Error>;
}

impl CheckLua for mlua::Lua {
    fn check_without_graph(&self, chunk: &str) -> Result<(), mlua::Error> {
        self.load(chunk).set_name("test chunk").exec()
    }

    fn check(&self, graph: &mut StackGraph, chunk: &str) -> Result<(), mlua::Error> {
        self.scope(|scope| {
            let graph = scope.create_userdata_ref_mut(graph);
            self.load(chunk).set_name("test chunk").call(graph)
        })
    }
}

#[test]
fn can_deepeq_from_lua() -> Result<(), anyhow::Error> {
    let l = new_lua();
    l.check_without_graph(
        r#"
          function check_deepeq(lhs, rhs, expected, expected_diffs)
            local actual, actual_diffs = deepeq(lhs, rhs)
            actual_diffs = table.concat(actual_diffs, ", ")
            assert_eq("deepeq", expected, actual)
            assert_eq("differences", expected_diffs, actual_diffs)
          end

          check_deepeq(0, 0, true, "")
          check_deepeq(0, 1, false, "different values for lhs (0) and rhs (1)")

          check_deepeq({"a", "b", "c"}, {"a", "b", "c"}, true, "")
          check_deepeq({"a", "b", "c"}, {"a", "b"}, false, "missing rhs.3")
          check_deepeq({"a", "b", "c"}, {"a", "b", "d"}, false, "different values for lhs.3 (c) and rhs.3 (d)")

          check_deepeq({a=1, b=2, c=3}, {a=1, b=2, c=3}, true, "")
          check_deepeq({a=1, b=2, c=3}, {a=1, b=2}, false, "missing rhs.c")
          check_deepeq({a=1, b=2, c=3}, {a=1, b=2, c=4}, false, "different values for lhs.c (3) and rhs.c (4)")
          check_deepeq({a=1, b=2, c=3}, {a=1, b=2, d=3}, false, "missing lhs.d, missing rhs.c")
        "#,
    )?;
    Ok(())
}

#[test]
fn can_create_nodes_from_lua() -> Result<(), anyhow::Error> {
    let l = new_lua();
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
    let l = new_lua();
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
    let l = new_lua();
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
    let l = new_lua();
    let mut graph = StackGraph::new();
    l.check(
        &mut graph,
        r#"
          local graph = ...
          local file = graph:file("test.py")
          local n0 = file:internal_scope_node()
          local n1 = file:internal_scope_node()
          n0:add_edge_to(n1)
          n0:add_edge_from(n1, 10)
        "#,
    )?;

    let file = graph.get_file("test.py").expect("Cannot find file");
    let n0 = graph
        .node_for_id(NodeID::new_in_file(file, 0))
        .expect("Cannot find node 0");
    let n1 = graph
        .node_for_id(NodeID::new_in_file(file, 1))
        .expect("Cannot find node 1");

    let edges_from_n0 = graph
        .outgoing_edges(n0)
        .map(|edge| (edge.sink, edge.precedence))
        .collect::<HashSet<_>>();
    assert_eq!(edges_from_n0, hashset! {(n1, 0)});

    let edges_from_n1 = graph
        .outgoing_edges(n1)
        .map(|edge| (edge.sink, edge.precedence))
        .collect::<HashSet<_>>();
    assert_eq!(edges_from_n1, hashset! {(n0, 10)});

    Ok(())
}

#[test]
fn can_create_all_node_types_from_lua() -> Result<(), anyhow::Error> {
    let l = new_lua();
    let mut graph = StackGraph::new();
    l.check(
        &mut graph,
        r#"
              local graph = ...
              local root = graph:root_node()
              local jump_to = graph:jump_to_node()
              local file = graph:file("test.py")
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
              local actual = {}
              for node in graph:nodes() do
                table.insert(actual, tostring(node))
              end
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
              }, actual)
            "#,
    )?;
    Ok(())
}
