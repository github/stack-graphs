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
pub mod sequenced_import_star;

/// An extension trait that makes it a bit easier to add stuff to our test stack graphs.
pub trait CreateStackGraph {
    fn definition(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node>;

    fn drop_scopes(&mut self, file: Handle<File>, local_id: u32) -> Handle<Node>;

    fn edge(&mut self, source: Handle<Node>, sink: Handle<Node>);

    fn exported_scope(&mut self, file: Handle<File>, local_id: u32) -> Handle<Node>;

    fn internal_scope(&mut self, file: Handle<File>, local_id: u32) -> Handle<Node>;

    fn pop_scoped_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node>;

    fn pop_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node>;

    fn push_scoped_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
        scope: Handle<Node>,
    ) -> Handle<Node>;

    fn push_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node>;

    fn reference(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node>;
}

impl CreateStackGraph for StackGraph {
    fn definition(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node> {
        let node = PopSymbolNode {
            id: NodeID { file, local_id },
            symbol,
            is_definition: true,
        };
        node.add_to_graph(self).expect("Duplicate node ID")
    }

    fn drop_scopes(&mut self, file: Handle<File>, local_id: u32) -> Handle<Node> {
        let node = DropScopesNode {
            id: NodeID { file, local_id },
        };
        node.add_to_graph(self).expect("Duplicate node ID")
    }

    fn edge(&mut self, source: Handle<Node>, sink: Handle<Node>) {
        let edge = Edge { source, sink };
        self.add_edge(edge);
    }

    fn exported_scope(&mut self, file: Handle<File>, local_id: u32) -> Handle<Node> {
        let node = ExportedScopeNode {
            id: NodeID { file, local_id },
        };
        node.add_to_graph(self).expect("Duplicate node ID")
    }

    fn internal_scope(&mut self, file: Handle<File>, local_id: u32) -> Handle<Node> {
        let node = InternalScopeNode {
            id: NodeID { file, local_id },
        };
        node.add_to_graph(self).expect("Duplicate node ID")
    }

    fn pop_scoped_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node> {
        let node = PopScopedSymbolNode {
            id: NodeID { file, local_id },
            symbol,
            is_definition: false,
        };
        node.add_to_graph(self).expect("Duplicate node ID")
    }

    fn pop_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node> {
        let node = PopSymbolNode {
            id: NodeID { file, local_id },
            symbol,
            is_definition: false,
        };
        node.add_to_graph(self).expect("Duplicate node ID")
    }

    fn push_scoped_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
        scope: Handle<Node>,
    ) -> Handle<Node> {
        let node = PushScopedSymbolNode {
            id: NodeID { file, local_id },
            symbol,
            scope,
            is_reference: false,
        };
        node.add_to_graph(self).expect("Duplicate node ID")
    }

    fn push_symbol(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node> {
        let node = PushSymbolNode {
            id: NodeID { file, local_id },
            symbol,
            is_reference: false,
        };
        node.add_to_graph(self).expect("Duplicate node ID")
    }

    fn reference(
        &mut self,
        file: Handle<File>,
        local_id: u32,
        symbol: Handle<Symbol>,
    ) -> Handle<Node> {
        let node = PushSymbolNode {
            id: NodeID { file, local_id },
            symbol,
            is_reference: true,
        };
        node.add_to_graph(self).expect("Duplicate node ID")
    }
}
