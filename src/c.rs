// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Defines a C API for working with stack graphs in other languages.

#![allow(non_camel_case_types)]

use libc::c_char;

use crate::arena::Handle;
use crate::graph::StackGraph;
use crate::graph::Symbol;

/// Contains all of the nodes and edges that make up a stack graph.
pub struct sg_stack_graph {
    inner: StackGraph,
}

/// Creates a new, initially empty stack graph.
#[no_mangle]
pub extern "C" fn sg_stack_graph_new() -> *mut sg_stack_graph {
    Box::into_raw(Box::new(sg_stack_graph {
        inner: StackGraph::new(),
    }))
}

/// Frees a stack graph, and all of its contents.
#[no_mangle]
pub extern "C" fn sg_stack_graph_free(graph: *mut sg_stack_graph) {
    drop(unsafe { Box::from_raw(graph) })
}

//-------------------------------------------------------------------------------------------------
// Symbols

/// A name that we are trying to resolve using stack graphs.
///
/// This typically represents a portion of an identifier as it appears in the source language.  It
/// can also represent some other "operation" that can occur in source code, and which needs to be
/// modeled in a stack graph — for instance, many languages will use a "fake" symbol named `.` to
/// represent member access.
#[repr(C)]
pub struct sg_symbol {
    pub symbol: *const c_char,
    pub symbol_len: usize,
}

/// A handle to a symbol in a stack graph.  A zero handle represents a missing symbol.
///
/// We deduplicate symbols in a stack graph — that is, we ensure that there are never multiple
/// `struct sg_symbol` instances with the same content.  That means that you can compare symbol
/// handles using simple equality, without having to dereference them.
pub type sg_symbol_handle = u32;

/// An array of all of the symbols in a stack graph.  Symbol handles are indices into this array.
/// There will never be a valid symbol at index 0; a handle with the value 0 represents a missing
/// symbol.
#[repr(C)]
pub struct sg_symbols {
    pub symbols: *const sg_symbol,
    pub count: usize,
}

/// Returns a reference to the array of symbol data in this stack graph.  The resulting array
/// pointer is only valid until the next call to any function that mutates the stack graph.
#[no_mangle]
pub extern "C" fn sg_stack_graph_symbols(graph: *const sg_stack_graph) -> sg_symbols {
    let graph = unsafe { &(*graph).inner };
    sg_symbols {
        symbols: graph.symbols.as_ptr() as *const sg_symbol,
        count: graph.symbols.len(),
    }
}

/// Adds new symbols to the stack graph.  You provide an array of symbol content, and an output
/// array, which must have the same length.  We will place each symbol's handle in the output
/// array.
///
/// We ensure that there is only ever one copy of a particular symbol stored in the graph — we
/// guarantee that identical symbols will have the same handles, meaning that you can compare the
/// handles using simple integer equality.
///
/// We copy the symbol data into the stack graph.  The symbol content you pass in does not need to
/// outlive the call to this function.
///
/// Each symbol must be a valid UTF-8 string.  If any symbol isn't valid UTF-8, it won't be added
/// to the stack graph, and the corresponding entry in the output array will be the null handle.
#[no_mangle]
pub extern "C" fn sg_stack_graph_add_symbols(
    graph: *mut sg_stack_graph,
    count: usize,
    symbols: *const *const c_char,
    lengths: *const usize,
    handles_out: *mut sg_symbol_handle,
) {
    let graph = unsafe { &mut (*graph).inner };
    let symbols = unsafe { std::slice::from_raw_parts(symbols as *const *const u8, count) };
    let lengths = unsafe { std::slice::from_raw_parts(lengths, count) };
    let handles_out = unsafe {
        std::slice::from_raw_parts_mut(handles_out as *mut Option<Handle<Symbol>>, count)
    };
    for i in 0..count {
        let symbol = unsafe { std::slice::from_raw_parts(symbols[i], lengths[i]) };
        handles_out[i] = match std::str::from_utf8(symbol) {
            Ok(symbol) => Some(graph.add_symbol(symbol)),
            Err(_) => None,
        };
    }
}
