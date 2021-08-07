// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use libc::c_char;
use stack_graphs::c::sg_edge;
use stack_graphs::c::sg_file_handle;
use stack_graphs::c::sg_node;
use stack_graphs::c::sg_node_handle;
use stack_graphs::c::sg_node_id;
use stack_graphs::c::sg_node_kind;
use stack_graphs::c::sg_stack_graph;
use stack_graphs::c::sg_stack_graph_add_edges;
use stack_graphs::c::sg_stack_graph_add_files;
use stack_graphs::c::sg_stack_graph_add_nodes;
use stack_graphs::c::sg_stack_graph_add_symbols;
use stack_graphs::c::sg_stack_graph_free;
use stack_graphs::c::sg_stack_graph_new;
use stack_graphs::c::sg_symbol_handle;
use stack_graphs::c::SG_JUMP_TO_NODE_HANDLE;
use stack_graphs::c::SG_ROOT_NODE_HANDLE;

use crate::test_graphs::CreateStackGraph;

pub struct TestGraph {
    pub graph: *mut sg_stack_graph,
}

impl Default for TestGraph {
    fn default() -> TestGraph {
        let graph = sg_stack_graph_new();
        TestGraph { graph }
    }
}

impl Drop for TestGraph {
    fn drop(&mut self) {
        sg_stack_graph_free(self.graph);
    }
}

impl TestGraph {
    fn add_node(&mut self, node: sg_node) -> sg_node_handle {
        let nodes = [node];
        let mut handles: [sg_node_handle; 1] = [0; 1];
        sg_stack_graph_add_nodes(
            self.graph,
            nodes.len(),
            nodes.as_ptr(),
            handles.as_mut_ptr(),
        );
        handles[0]
    }
}

impl CreateStackGraph for TestGraph {
    type File = sg_file_handle;
    type Node = sg_node_handle;
    type Symbol = sg_symbol_handle;

    fn definition(
        &mut self,
        file: sg_file_handle,
        local_id: u32,
        symbol: sg_symbol_handle,
    ) -> sg_node_handle {
        self.add_node(sg_node {
            kind: sg_node_kind::SG_NODE_KIND_POP_SYMBOL,
            id: sg_node_id { file, local_id },
            symbol,
            is_clickable: true,
            scope: sg_node_id::default(),
        })
    }

    fn drop_scopes(&mut self, file: sg_file_handle, local_id: u32) -> sg_node_handle {
        self.add_node(sg_node {
            kind: sg_node_kind::SG_NODE_KIND_DROP_SCOPES,
            id: sg_node_id { file, local_id },
            symbol: 0,
            is_clickable: false,
            scope: sg_node_id::default(),
        })
    }

    fn edge(&mut self, source: sg_node_handle, sink: sg_node_handle) {
        let edge = sg_edge {
            source,
            sink,
            precedence: 0,
        };
        let edges = [edge];
        sg_stack_graph_add_edges(self.graph, edges.len(), edges.as_ptr());
    }

    fn exported_scope(&mut self, file: sg_file_handle, local_id: u32) -> sg_node_handle {
        self.add_node(sg_node {
            kind: sg_node_kind::SG_NODE_KIND_EXPORTED_SCOPE,
            id: sg_node_id { file, local_id },
            symbol: 0,
            is_clickable: false,
            scope: sg_node_id::default(),
        })
    }

    fn file(&mut self, name: &str) -> sg_file_handle {
        let names = [name.as_bytes().as_ptr() as *const c_char];
        let lengths = [name.len()];
        let mut handles: [sg_file_handle; 1] = [0; 1];
        sg_stack_graph_add_files(
            self.graph,
            names.len(),
            names.as_ptr(),
            lengths.as_ptr(),
            handles.as_mut_ptr(),
        );
        handles[0]
    }

    fn internal_scope(&mut self, file: sg_file_handle, local_id: u32) -> sg_node_handle {
        self.add_node(sg_node {
            kind: sg_node_kind::SG_NODE_KIND_INTERNAL_SCOPE,
            id: sg_node_id { file, local_id },
            symbol: 0,
            is_clickable: false,
            scope: sg_node_id::default(),
        })
    }

    fn jump_to_node(&mut self) -> sg_node_handle {
        SG_JUMP_TO_NODE_HANDLE
    }

    fn pop_scoped_symbol(
        &mut self,
        file: sg_file_handle,
        local_id: u32,
        symbol: sg_symbol_handle,
    ) -> sg_node_handle {
        self.add_node(sg_node {
            kind: sg_node_kind::SG_NODE_KIND_POP_SCOPED_SYMBOL,
            id: sg_node_id { file, local_id },
            symbol,
            is_clickable: false,
            scope: sg_node_id::default(),
        })
    }

    fn pop_symbol(
        &mut self,
        file: sg_file_handle,
        local_id: u32,
        symbol: sg_symbol_handle,
    ) -> sg_node_handle {
        self.add_node(sg_node {
            kind: sg_node_kind::SG_NODE_KIND_POP_SYMBOL,
            id: sg_node_id { file, local_id },
            symbol,
            is_clickable: false,
            scope: sg_node_id::default(),
        })
    }

    fn push_scoped_symbol(
        &mut self,
        file: sg_file_handle,
        local_id: u32,
        symbol: sg_symbol_handle,
        scope_file: sg_file_handle,
        scope_id: u32,
    ) -> sg_node_handle {
        let scope = sg_node_id {
            file: scope_file,
            local_id: scope_id,
        };
        self.add_node(sg_node {
            kind: sg_node_kind::SG_NODE_KIND_PUSH_SCOPED_SYMBOL,
            id: sg_node_id { file, local_id },
            symbol,
            is_clickable: false,
            scope,
        })
    }

    fn push_symbol(
        &mut self,
        file: sg_file_handle,
        local_id: u32,
        symbol: sg_symbol_handle,
    ) -> sg_node_handle {
        self.add_node(sg_node {
            kind: sg_node_kind::SG_NODE_KIND_PUSH_SYMBOL,
            id: sg_node_id { file, local_id },
            symbol,
            is_clickable: false,
            scope: sg_node_id::default(),
        })
    }

    fn reference(
        &mut self,
        file: sg_file_handle,
        local_id: u32,
        symbol: sg_symbol_handle,
    ) -> sg_node_handle {
        self.add_node(sg_node {
            kind: sg_node_kind::SG_NODE_KIND_PUSH_SYMBOL,
            id: sg_node_id { file, local_id },
            symbol,
            is_clickable: true,
            scope: sg_node_id::default(),
        })
    }

    fn root_node(&mut self) -> sg_node_handle {
        SG_ROOT_NODE_HANDLE
    }

    fn symbol(&mut self, value: &str) -> sg_symbol_handle {
        let symbols = [value.as_bytes().as_ptr() as *const c_char];
        let lengths = [value.len()];
        let mut handles: [sg_symbol_handle; 1] = [0; 1];
        sg_stack_graph_add_symbols(
            self.graph,
            symbols.len(),
            symbols.as_ptr(),
            lengths.as_ptr(),
            handles.as_mut_ptr(),
        );
        handles[0]
    }
}
