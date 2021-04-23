// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Contains several useful stack graphs that can be used in test cases.

use stack_graphs::arena::Handle;
use stack_graphs::graph::*;

pub mod class_field_through_function_parameter;
pub mod cyclic_imports_python;
pub mod cyclic_imports_rust;
pub mod sequenced_import_star;

/// An extension trait that makes it a bit easier to add stuff to our test stack graphs.
pub trait CreateStackGraph {
    type File: Clone + Copy;
    type Node: Clone + Copy;
    type Symbol: Clone + Copy;

    fn definition(&mut self, file: Self::File, local_id: u32, symbol: Self::Symbol) -> Self::Node;

    fn drop_scopes(&mut self, file: Self::File, local_id: u32) -> Self::Node;

    fn edge(&mut self, source: Self::Node, sink: Self::Node);

    fn exported_scope(&mut self, file: Self::File, local_id: u32) -> Self::Node;

    fn file(&mut self, name: &str) -> Self::File;

    fn internal_scope(&mut self, file: Self::File, local_id: u32) -> Self::Node;

    fn jump_to_node(&mut self) -> Self::Node;

    fn pop_scoped_symbol(
        &mut self,
        file: Self::File,
        local_id: u32,
        symbol: Self::Symbol,
    ) -> Self::Node;

    fn pop_symbol(&mut self, file: Self::File, local_id: u32, symbol: Self::Symbol) -> Self::Node;

    fn push_scoped_symbol(
        &mut self,
        file: Self::File,
        local_id: u32,
        symbol: Self::Symbol,
        scope: Self::Node,
    ) -> Self::Node;

    fn push_symbol(&mut self, file: Self::File, local_id: u32, symbol: Self::Symbol) -> Self::Node;

    fn reference(&mut self, file: Self::File, local_id: u32, symbol: Self::Symbol) -> Self::Node;

    fn root_node(&mut self) -> Self::Node;

    fn symbol(&mut self, value: &str) -> Self::Symbol;
}

impl CreateStackGraph for StackGraph {
    type File = Handle<File>;
    type Node = Handle<Node>;
    type Symbol = Handle<Symbol>;

    fn definition(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node> {
        self.add_pop_symbol_node(NodeID::new_in_file(file, local_id), symbol, true)
            .expect("Duplicate node ID")
    }

    fn drop_scopes(&mut self, file: Handle<File>, local_id: u32) -> Handle<Node> {
        self.add_drop_scopes_node(NodeID::new_in_file(file, local_id))
            .expect("Duplicate node ID")
    }

    fn edge(&mut self, source: Handle<Node>, sink: Handle<Node>) {
        self.add_edge(source, sink, 0);
    }

    fn exported_scope(&mut self, file: Handle<File>, local_id: u32) -> Handle<Node> {
        self.add_exported_scope_node(NodeID::new_in_file(file, local_id))
            .expect("Duplicate node ID")
    }

    fn file(&mut self, name: &str) -> Handle<File> {
        self.get_or_create_file(name)
    }

    fn internal_scope(&mut self, file: Handle<File>, local_id: u32) -> Handle<Node> {
        self.add_internal_scope_node(NodeID::new_in_file(file, local_id))
            .expect("Duplicate node ID")
    }

    fn jump_to_node(&mut self) -> Handle<Node> {
        StackGraph::jump_to_node(self)
    }

    fn pop_scoped_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node> {
        self.add_pop_scoped_symbol_node(NodeID::new_in_file(file, local_id), symbol, false)
            .expect("Duplicate node ID")
    }

    fn pop_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node> {
        self.add_pop_symbol_node(NodeID::new_in_file(file, local_id), symbol, false)
            .expect("Duplicate node ID")
    }

    fn push_scoped_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
        scope: Handle<Node>,
    ) -> Handle<Node> {
        self.add_push_scoped_symbol_node(NodeID::new_in_file(file, local_id), symbol, scope, false)
            .expect("Duplicate node ID")
    }

    fn push_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node> {
        self.add_push_symbol_node(NodeID::new_in_file(file, local_id), symbol, false)
            .expect("Duplicate node ID")
    }

    fn reference(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node> {
        self.add_push_symbol_node(NodeID::new_in_file(file, local_id), symbol, true)
            .expect("Duplicate node ID")
    }

    fn root_node(&mut self) -> Handle<Node> {
        StackGraph::root_node(self)
    }

    fn symbol(&mut self, value: &str) -> Handle<Symbol> {
        self.add_symbol(value)
    }
}
