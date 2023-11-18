// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

#![cfg_attr(docsrs, doc(cfg(feature = "lua")))]
//! Provides access to `StackGraph` instances from Lua.
//!
//! With the `lua` feature enabled, you can add [`StackGraph`] instances to a [`Lua`][mlua::Lua]
//! interpreter.  You might typically use this to _create_ stack graphs from Lua, by calling a Lua
//! function with an empty stack graph as a parameter.  Note that you'll almost certainly need to
//! use `mlua`'s [scoped values](mlua::Lua::scope) mechanism so that you can still use the
//! [`StackGraph`] on the Rust side once the Lua function has finished.
//!
//! ```
//! # use mlua::Lua;
//! # use stack_graphs::graph::StackGraph;
//! # fn main() -> Result<(), mlua::Error> {
//! let lua = Lua::new();
//! let chunk = r#"
//!     function process_graph(graph)
//!       local file = graph:file("test.py")
//!       local def = file:definition_node("foo")
//!       def:add_edge_from(graph:root_node())
//!     end
//! "#;
//! lua.load(chunk).set_name("stack graph chunk").exec()?;
//! let process_graph: mlua::Function = lua.globals().get("process_graph")?;
//!
//! let mut graph = StackGraph::new();
//! lua.scope(|scope| {
//!     let graph = scope.create_userdata_ref_mut(&mut graph);
//!     process_graph.call(graph)
//! })?;
//! assert_eq!(graph.iter_nodes().count(), 3);
//! # Ok(())
//! # }
//! ```
//!
//! ## Building
//!
//! Lua support is only enabled if you compile with the `lua` feature.  This feature is not enough
//! on its own, because the `mlua` crate supports multiple Lua versions, and can either link
//! against a system-installed copy of Lua, or build its own copy from vendored Lua source.  These
//! choices are all controlled via additional features on the `mlua` crate.
//!
//! When building and testing this crate, make sure to provide all necessary features on the
//! command line:
//!
//! ``` console
//! $ cargo test --features lua,mlua/lua54,mlua/vendored
//! ```
//!
//! When building a crate that depends on this crate, add a dependency on `mlua` so that you can
//! set its feature flags:
//!
//! ``` toml
//! [dependencies]
//! stack-graphs = { version="0.13", features=["lua"] }
//! mlua = { version="0.9", features=["lua54", "vendored"] }
//! ```

// Implementation notes: Stack graphs, files, and nodes can live inside the Lua interpreter as
// objects.  They are each wrapped in a userdata, with a metatable defining the methods that are
// available.  With mlua, the UserData trait is the way to define these metatables and methods.
//
// Complicating matters is that files and nodes need to be represented by a _pair_ of Lua values:
// the handle of the file or node, and a reference to the StackGraph that the file or node lives
// in.  We need both because some of the methods need to dereference the handle to get e.g. the
// `Node` instance.  It's not safe to dereference the handle when we create the userdata, because
// the resulting pointer is not guaranteed to be stable.  (If you add another node, the arena's
// storage might get resized, moving the node instances around in memory.)
//
// To handle this, we leverage Lua's ability to associate “user values” with each userdata.  For
// files and nodes, we store the graph's userdata (i.e. its Lua representation) as the user value
// of each file and node userdata.
//
// That, in turn, means that we must use `add_function` to define each metatable method, since that
// gives us an `mlua::AnyUserData`, which lets us access the userdata's underlying Rust value _and_
// its user value.  (Typically, you would use the more ergonomic `add_method` or `add_method_mut`,
// which take care of unwrapping the userdata and giving you a &ref or &mut ref to the underlying
// Rust type.  But then you don't have access to the userdata's user value.)

use std::fmt::Write;
use std::num::NonZeroU32;

use controlled_option::ControlledOption;
use lsp_positions::Span;
use mlua::AnyUserData;
use mlua::UserData;
use mlua::UserDataMethods;

use crate::arena::Handle;
use crate::graph::File;
use crate::graph::Node;
use crate::graph::StackGraph;

impl UserData for StackGraph {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("file", |l, (graph_ud, name): (AnyUserData, String)| {
            let file = {
                let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                graph.get_or_create_file(&name)
            };
            let file_ud = l.create_userdata(file)?;
            file_ud.set_user_value(graph_ud)?;
            Ok(file_ud)
        });

        methods.add_function("jump_to_node", |l, graph_ud: AnyUserData| {
            let node = StackGraph::jump_to_node();
            let node_ud = l.create_userdata(node)?;
            node_ud.set_user_value(graph_ud)?;
            Ok(node_ud)
        });

        methods.add_function("nodes", |l, graph_ud: AnyUserData| {
            let iter = l.create_function(
                |l, (graph_ud, prev_node_ud): (AnyUserData, Option<AnyUserData>)| {
                    let prev_index = match prev_node_ud {
                        Some(prev_node_ud) => {
                            let prev_node = prev_node_ud.borrow::<Handle<Node>>()?;
                            prev_node.as_u32()
                        }
                        None => 0,
                    };
                    let node_index = {
                        let graph = graph_ud.borrow::<StackGraph>()?;
                        let node_count = graph.nodes.len() as u32;
                        if prev_index == node_count - 1 {
                            return Ok(None);
                        }
                        unsafe { NonZeroU32::new_unchecked(prev_index + 1) }
                    };
                    let node = Handle::new(node_index);
                    let node_ud = l.create_userdata::<Handle<Node>>(node)?;
                    node_ud.set_user_value(graph_ud)?;
                    Ok(Some(node_ud))
                },
            )?;
            Ok((iter, graph_ud, None::<AnyUserData>))
        });

        methods.add_function("root_node", |l, graph_ud: AnyUserData| {
            let node = StackGraph::root_node();
            let node_ud = l.create_userdata(node)?;
            node_ud.set_user_value(graph_ud)?;
            Ok(node_ud)
        });
    }
}

impl UserData for Handle<File> {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function(
            "definition_node",
            |l, (file_ud, symbol): (AnyUserData, String)| {
                let file = *file_ud.borrow::<Handle<File>>()?;
                let graph_ud = file_ud.user_value::<AnyUserData>()?;
                let node = {
                    let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                    let symbol = graph.add_symbol(&symbol);
                    let node_id = graph.new_node_id(file);
                    graph
                        .add_pop_symbol_node(node_id, symbol, true)
                        .expect("Node ID collision")
                };
                let node_ud = l.create_userdata(node)?;
                node_ud.set_user_value(graph_ud)?;
                Ok(node_ud)
            },
        );

        methods.add_function("drop_scopes_node", |l, file_ud: AnyUserData| {
            let file = *file_ud.borrow::<Handle<File>>()?;
            let graph_ud = file_ud.user_value::<AnyUserData>()?;
            let node = {
                let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                let node_id = graph.new_node_id(file);
                graph
                    .add_drop_scopes_node(node_id)
                    .expect("Node ID collision")
            };
            let node_ud = l.create_userdata(node)?;
            node_ud.set_user_value(graph_ud)?;
            Ok(node_ud)
        });

        methods.add_function("exported_scope_node", |l, file_ud: AnyUserData| {
            let file = *file_ud.borrow::<Handle<File>>()?;
            let graph_ud = file_ud.user_value::<AnyUserData>()?;
            let node = {
                let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                let node_id = graph.new_node_id(file);
                graph
                    .add_scope_node(node_id, true)
                    .expect("Node ID collision")
            };
            let node_ud = l.create_userdata(node)?;
            node_ud.set_user_value(graph_ud)?;
            Ok(node_ud)
        });

        methods.add_function("internal_scope_node", |l, file_ud: AnyUserData| {
            let file = *file_ud.borrow::<Handle<File>>()?;
            let graph_ud = file_ud.user_value::<AnyUserData>()?;
            let node = {
                let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                let node_id = graph.new_node_id(file);
                graph
                    .add_scope_node(node_id, false)
                    .expect("Node ID collision")
            };
            let node_ud = l.create_userdata(node)?;
            node_ud.set_user_value(graph_ud)?;
            Ok(node_ud)
        });

        methods.add_function(
            "pop_scoped_symbol_node",
            |l, (file_ud, symbol): (AnyUserData, String)| {
                let file = *file_ud.borrow::<Handle<File>>()?;
                let graph_ud = file_ud.user_value::<AnyUserData>()?;
                let node = {
                    let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                    let symbol = graph.add_symbol(&symbol);
                    let node_id = graph.new_node_id(file);
                    graph
                        .add_pop_scoped_symbol_node(node_id, symbol, false)
                        .expect("Node ID collision")
                };
                let node_ud = l.create_userdata(node)?;
                node_ud.set_user_value(graph_ud)?;
                Ok(node_ud)
            },
        );

        methods.add_function(
            "pop_symbol_node",
            |l, (file_ud, symbol): (AnyUserData, String)| {
                let file = *file_ud.borrow::<Handle<File>>()?;
                let graph_ud = file_ud.user_value::<AnyUserData>()?;
                let node = {
                    let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                    let symbol = graph.add_symbol(&symbol);
                    let node_id = graph.new_node_id(file);
                    graph
                        .add_pop_symbol_node(node_id, symbol, false)
                        .expect("Node ID collision")
                };
                let node_ud = l.create_userdata(node)?;
                node_ud.set_user_value(graph_ud)?;
                Ok(node_ud)
            },
        );

        methods.add_function(
            "push_scoped_symbol_node",
            |l, (file_ud, symbol, scope_ud): (AnyUserData, String, AnyUserData)| {
                let file = *file_ud.borrow::<Handle<File>>()?;
                let graph_ud = file_ud.user_value::<AnyUserData>()?;
                let scope = *scope_ud.borrow::<Handle<Node>>()?;
                let node = {
                    let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                    let scope_id = {
                        let scope = &graph[scope];
                        if !scope.is_exported_scope() {
                            return Err(mlua::Error::RuntimeError(
                                "Can only push exported scope nodes".to_string(),
                            ));
                        }
                        scope.id()
                    };
                    let symbol = graph.add_symbol(&symbol);
                    let node_id = graph.new_node_id(file);
                    graph
                        .add_push_scoped_symbol_node(node_id, symbol, scope_id, false)
                        .expect("Node ID collision")
                };
                let node_ud = l.create_userdata(node)?;
                node_ud.set_user_value(graph_ud)?;
                Ok(node_ud)
            },
        );

        methods.add_function(
            "push_symbol_node",
            |l, (file_ud, symbol): (AnyUserData, String)| {
                let file = *file_ud.borrow::<Handle<File>>()?;
                let graph_ud = file_ud.user_value::<AnyUserData>()?;
                let node = {
                    let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                    let symbol = graph.add_symbol(&symbol);
                    let node_id = graph.new_node_id(file);
                    graph
                        .add_push_symbol_node(node_id, symbol, false)
                        .expect("Node ID collision")
                };
                let node_ud = l.create_userdata(node)?;
                node_ud.set_user_value(graph_ud)?;
                Ok(node_ud)
            },
        );

        methods.add_function(
            "reference_node",
            |l, (file_ud, symbol): (AnyUserData, String)| {
                let file = *file_ud.borrow::<Handle<File>>()?;
                let graph_ud = file_ud.user_value::<AnyUserData>()?;
                let node = {
                    let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                    let symbol = graph.add_symbol(&symbol);
                    let node_id = graph.new_node_id(file);
                    graph
                        .add_push_symbol_node(node_id, symbol, true)
                        .expect("Node ID collision")
                };
                let node_ud = l.create_userdata(node)?;
                node_ud.set_user_value(graph_ud)?;
                Ok(node_ud)
            },
        );

        methods.add_function(
            "scoped_definition_node",
            |l, (file_ud, symbol): (AnyUserData, String)| {
                let file = *file_ud.borrow::<Handle<File>>()?;
                let graph_ud = file_ud.user_value::<AnyUserData>()?;
                let node = {
                    let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                    let symbol = graph.add_symbol(&symbol);
                    let node_id = graph.new_node_id(file);
                    graph
                        .add_pop_scoped_symbol_node(node_id, symbol, true)
                        .expect("Node ID collision")
                };
                let node_ud = l.create_userdata(node)?;
                node_ud.set_user_value(graph_ud)?;
                Ok(node_ud)
            },
        );

        methods.add_function(
            "scoped_reference_node",
            |l, (file_ud, symbol, scope_ud): (AnyUserData, String, AnyUserData)| {
                let file = *file_ud.borrow::<Handle<File>>()?;
                let graph_ud = file_ud.user_value::<AnyUserData>()?;
                let scope = *scope_ud.borrow::<Handle<Node>>()?;
                let node = {
                    let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                    let scope_id = {
                        let scope = &graph[scope];
                        if !scope.is_exported_scope() {
                            return Err(mlua::Error::RuntimeError(
                                "Can only push exported scope nodes".to_string(),
                            ));
                        }
                        scope.id()
                    };
                    let symbol = graph.add_symbol(&symbol);
                    let node_id = graph.new_node_id(file);
                    graph
                        .add_push_scoped_symbol_node(node_id, symbol, scope_id, true)
                        .expect("Node ID collision")
                };
                let node_ud = l.create_userdata(node)?;
                node_ud.set_user_value(graph_ud)?;
                Ok(node_ud)
            },
        );
    }
}

impl UserData for Handle<Node> {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function(
            "add_edge_from",
            |_, (this_ud, from_ud, precedence): (AnyUserData, AnyUserData, Option<i32>)| {
                let this = *this_ud.borrow::<Handle<Node>>()?;
                let from = *from_ud.borrow::<Handle<Node>>()?;
                let graph_ud = this_ud.user_value::<AnyUserData>()?;
                let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                let precedence = precedence.unwrap_or(0);
                graph.add_edge(from, this, precedence);
                Ok(())
            },
        );

        methods.add_function(
            "add_edge_to",
            |_, (this_ud, to_ud, precedence): (AnyUserData, AnyUserData, Option<i32>)| {
                let this = *this_ud.borrow::<Handle<Node>>()?;
                let to = *to_ud.borrow::<Handle<Node>>()?;
                let graph_ud = this_ud.user_value::<AnyUserData>()?;
                let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                let precedence = precedence.unwrap_or(0);
                graph.add_edge(this, to, precedence);
                Ok(())
            },
        );

        methods.add_function("debug_info", |l, node_ud: AnyUserData| {
            let node = *node_ud.borrow::<Handle<Node>>()?;
            let graph_ud = node_ud.user_value::<AnyUserData>()?;
            let graph = graph_ud.borrow::<StackGraph>()?;
            let debug_info = match graph.node_debug_info(node) {
                Some(debug_info) => debug_info,
                None => return Ok(None),
            };
            let result = l.create_table()?;
            for entry in debug_info.iter() {
                result.set(&graph[entry.key], &graph[entry.value])?;
            }
            Ok(Some(result))
        });

        methods.add_function("definiens_span", |_, node_ud: AnyUserData| {
            let node = *node_ud.borrow::<Handle<Node>>()?;
            let graph_ud = node_ud.user_value::<AnyUserData>()?;
            let graph = graph_ud.borrow::<StackGraph>()?;
            let source_info = match graph.source_info(node) {
                Some(source_info) => source_info,
                None => return Ok(None),
            };
            Ok(Some(source_info.definiens_span.clone()))
        });

        methods.add_function("local_id", |_, node_ud: AnyUserData| {
            let node = *node_ud.borrow::<Handle<Node>>()?;
            let graph_ud = node_ud.user_value::<AnyUserData>()?;
            let graph = graph_ud.borrow::<StackGraph>()?;
            Ok(graph[node].id().local_id())
        });

        methods.add_function(
            "set_debug_info",
            |_, (node_ud, k, v): (AnyUserData, String, String)| {
                let node = *node_ud.borrow::<Handle<Node>>()?;
                let graph_ud = node_ud.user_value::<AnyUserData>()?;
                let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                let k = graph.add_string(&k);
                let v = graph.add_string(&v);
                graph.node_debug_info_mut(node).add(k, v);
                Ok(())
            },
        );

        methods.add_function(
            "set_definiens_span",
            |_, (node_ud, definiens_span): (AnyUserData, Span)| {
                let node = *node_ud.borrow::<Handle<Node>>()?;
                let graph_ud = node_ud.user_value::<AnyUserData>()?;
                let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                graph.source_info_mut(node).definiens_span = definiens_span;
                Ok(())
            },
        );

        methods.add_function("set_span", |_, (node_ud, span): (AnyUserData, Span)| {
            let node = *node_ud.borrow::<Handle<Node>>()?;
            let graph_ud = node_ud.user_value::<AnyUserData>()?;
            let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
            graph.source_info_mut(node).span = span;
            Ok(())
        });

        methods.add_function(
            "set_syntax_type",
            |_, (node_ud, syntax_type): (AnyUserData, String)| {
                let node = *node_ud.borrow::<Handle<Node>>()?;
                let graph_ud = node_ud.user_value::<AnyUserData>()?;
                let mut graph = graph_ud.borrow_mut::<StackGraph>()?;
                let syntax_type = graph.add_string(&syntax_type);
                graph.source_info_mut(node).syntax_type = ControlledOption::some(syntax_type);
                Ok(())
            },
        );

        methods.add_function("span", |_, node_ud: AnyUserData| {
            let node = *node_ud.borrow::<Handle<Node>>()?;
            let graph_ud = node_ud.user_value::<AnyUserData>()?;
            let graph = graph_ud.borrow::<StackGraph>()?;
            let source_info = match graph.source_info(node) {
                Some(source_info) => source_info,
                None => return Ok(None),
            };
            Ok(Some(source_info.span.clone()))
        });

        methods.add_function("syntax_type", |_, node_ud: AnyUserData| {
            let node = *node_ud.borrow::<Handle<Node>>()?;
            let graph_ud = node_ud.user_value::<AnyUserData>()?;
            let graph = graph_ud.borrow::<StackGraph>()?;
            let source_info = match graph.source_info(node) {
                Some(source_info) => source_info,
                None => return Ok(None),
            };
            let syntax_type = match source_info.syntax_type.into_option() {
                Some(syntax_type) => syntax_type,
                None => return Ok(None),
            };
            Ok(Some(graph[syntax_type].to_string()))
        });

        methods.add_meta_function(mlua::MetaMethod::ToString, |_, node_ud: AnyUserData| {
            let node = *node_ud.borrow::<Handle<Node>>()?;
            let graph_ud = node_ud.user_value::<AnyUserData>()?;
            let graph = graph_ud.borrow::<StackGraph>()?;
            let mut display = graph[node].display(&graph).to_string();
            if let Some(source_info) = graph.source_info(node) {
                display.pop(); // remove the trailing ]
                if let Some(syntax_type) = source_info.syntax_type.into_option() {
                    write!(&mut display, " ({})", syntax_type.display(&graph)).unwrap();
                }
                if source_info.span != Span::default() {
                    write!(
                        &mut display,
                        " at {}:{}-{}:{}",
                        source_info.span.start.line,
                        source_info.span.start.column.utf8_offset,
                        source_info.span.end.line,
                        source_info.span.end.column.utf8_offset,
                    )
                    .unwrap();
                }
                if source_info.definiens_span != Span::default() {
                    write!(
                        &mut display,
                        " def {}:{}-{}:{}",
                        source_info.definiens_span.start.line,
                        source_info.definiens_span.start.column.utf8_offset,
                        source_info.definiens_span.end.line,
                        source_info.definiens_span.end.column.utf8_offset,
                    )
                    .unwrap();
                }
                display.push(']');
            }
            Ok(display)
        });
    }
}
