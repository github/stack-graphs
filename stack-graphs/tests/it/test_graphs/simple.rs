// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2022, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use lsp_positions::Offset;
use lsp_positions::Position;
use lsp_positions::Span;
use stack_graphs::graph::SourceInfo;
use stack_graphs::graph::StackGraph;
use std::ops::Range;

use crate::test_graphs::CreateStackGraph;

/// A minimal stack graph containing data of all types.

#[allow(non_snake_case)]
pub fn new() -> StackGraph {
    let mut graph = StackGraph::default();
    let root = graph.root_node();
    let jump_to = graph.jump_to_node();

    let sym_x = graph.symbol("x");
    let sym_call = graph.symbol("()");
    let sym_dot = graph.symbol(".");

    let file = graph.file("test.py");
    let ref_x = graph.reference(file, 1, sym_x);
    let push_dot = graph.push_symbol(file, 2, sym_dot);
    let scope_x = graph.exported_scope(file, 3);
    let push_call = graph.push_scoped_symbol(file, 4, sym_call, file, 3);
    let scope = graph.internal_scope(file, 5);
    let pop_call = graph.pop_scoped_symbol(file, 6, sym_call);
    let drop = graph.drop_scopes(file, 7);
    let pop_dot = graph.pop_symbol(file, 8, sym_dot);
    let def_x = graph.definition(file, 9, sym_x);

    graph.edge(ref_x, push_dot);
    graph.edge(push_dot, push_call);
    graph.edge(scope_x, pop_dot);
    graph.edge(push_call, scope);
    graph.edge(scope, pop_call);
    graph.edge(scope, root);
    graph.add_edge(pop_call, jump_to, 1);
    graph.add_edge(pop_call, drop, 0);
    graph.edge(drop, pop_dot);
    graph.edge(pop_dot, def_x);

    let str_var = graph.add_string("variable");
    let str_line0 = graph.add_string("x = 42");
    let str_line1 = graph.add_string("print(x)");
    *graph.source_info_mut(def_x) = SourceInfo {
        span: Span {
            start: Position {
                line: 0,
                column: Offset {
                    utf8_offset: 0,
                    utf16_offset: 0,
                    grapheme_offset: 0,
                },
                containing_line: Range { start: 0, end: 6 },
                trimmed_line: Range { start: 0, end: 6 },
            },
            end: Position {
                line: 0,
                column: Offset {
                    utf8_offset: 1,
                    utf16_offset: 1,
                    grapheme_offset: 1,
                },
                containing_line: Range { start: 0, end: 6 },
                trimmed_line: Range { start: 0, end: 6 },
            },
        },
        syntax_type: str_var.into(),
        containing_line: str_line0.into(),
        definiens_span: Span::default(),
    };
    *graph.source_info_mut(ref_x) = SourceInfo {
        span: Span {
            start: Position {
                line: 1,
                column: Offset {
                    utf8_offset: 13,
                    utf16_offset: 13,
                    grapheme_offset: 13,
                },
                containing_line: Range { start: 7, end: 15 },
                trimmed_line: Range { start: 7, end: 15 },
            },
            end: Position {
                line: 1,
                column: Offset {
                    utf8_offset: 14,
                    utf16_offset: 14,
                    grapheme_offset: 14,
                },
                containing_line: Range { start: 7, end: 15 },
                trimmed_line: Range { start: 7, end: 15 },
            },
        },
        syntax_type: str_var.into(),
        containing_line: str_line1.into(),
        definiens_span: Span::default(),
    };

    let str_dsl_var = graph.add_string("dsl_var");
    let str_dsl_position = graph.add_string("dsl_position");
    let str_arg_scope = graph.add_string("arg_scope");
    let str_lexical_scope = graph.add_string("lexical_scope");
    let str_pos_one = graph.add_string("line 31 column 20");
    let str_pos_two = graph.add_string("line 8 column 11");
    let str_pos_three = graph.add_string("line 23 column 4");
    graph
        .node_debug_info_mut(scope_x)
        .add(str_dsl_var, str_arg_scope);
    graph
        .node_debug_info_mut(scope_x)
        .add(str_dsl_position, str_pos_one);
    graph
        .node_debug_info_mut(scope)
        .add(str_dsl_var, str_lexical_scope);
    graph
        .node_debug_info_mut(scope)
        .add(str_dsl_position, str_pos_two);
    graph
        .edge_debug_info_mut(scope, root)
        .add(str_dsl_position, str_pos_three);

    graph
}
