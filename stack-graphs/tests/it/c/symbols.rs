// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use controlled_option::ControlledOption;
use libc::c_char;
use stack_graphs::arena::Handle;
use stack_graphs::c::sg_stack_graph_add_symbols;
use stack_graphs::c::sg_stack_graph_free;
use stack_graphs::c::sg_stack_graph_new;
use stack_graphs::c::sg_stack_graph_symbols;
use stack_graphs::c::sg_symbol_handle;
use stack_graphs::c::sg_symbols;
use stack_graphs::c::SG_ARENA_CHUNK_SIZE;
use stack_graphs::c::SG_NULL_HANDLE;
use stack_graphs::graph::Symbol;

fn lengths(data: &[&'static str]) -> Vec<usize> {
    data.iter().map(|s| s.len()).collect()
}

fn index_chunked<'a, T>(chunks: *const *const T, index: usize) -> &'a T {
    let chunk_index = index / SG_ARENA_CHUNK_SIZE;
    let item_index = index % SG_ARENA_CHUNK_SIZE;
    let chunks = unsafe { std::slice::from_raw_parts(chunks, chunk_index + 1) };
    let chunk = chunks[chunk_index];
    let items = unsafe { std::slice::from_raw_parts(chunk, item_index + 1) };
    &items[item_index]
}

fn get_symbol(arena: &sg_symbols, handle: Handle<Symbol>) -> &str {
    let symbol = index_chunked(arena.symbols, handle.as_usize());
    unsafe {
        let bytes = std::slice::from_raw_parts(symbol.symbol as *const u8, symbol.symbol_len);
        std::str::from_utf8_unchecked(bytes)
    }
}

#[test]
fn can_create_symbols() {
    let graph = sg_stack_graph_new();

    let symbol_data = ["a", "a", "b", "c"];
    let mut handles: [Option<Handle<Symbol>>; 4] = [None; 4];
    sg_stack_graph_add_symbols(
        graph,
        symbol_data.len(),
        symbol_data.join("").as_ptr() as *const c_char,
        lengths(&symbol_data).as_ptr(),
        handles.as_mut_ptr() as *mut sg_symbol_handle,
    );

    // All of the symbols should have been created successfully
    assert!(handles.as_ref().iter().all(|h| h.is_some()));

    // The handles should be comparable.
    let a1 = handles[0].unwrap();
    let a2 = handles[1].unwrap();
    let b = handles[2].unwrap();
    let c = handles[3].unwrap();
    assert_eq!(a1, a2);
    assert_ne!(a1, b);
    assert_ne!(a1, c);
    assert_ne!(a2, b);
    assert_ne!(a2, c);
    assert_ne!(b, c);

    // We should be able to dereference into the symbols arena to get the symbol content.
    let symbol_arena = sg_stack_graph_symbols(graph);
    assert_eq!(get_symbol(&symbol_arena, a1), "a");
    assert_eq!(get_symbol(&symbol_arena, a2), "a");
    assert_eq!(get_symbol(&symbol_arena, b), "b");
    assert_eq!(get_symbol(&symbol_arena, c), "c");

    sg_stack_graph_free(graph);
}

#[test]
#[allow(unused_assignments)]
fn verify_null_symbol_representation() {
    let bytes = [0x55u8; std::mem::size_of::<Handle<Symbol>>()];
    let mut rust: ControlledOption<Handle<Symbol>> = unsafe { std::mem::transmute(bytes) };
    rust = ControlledOption::none();
    let c: sg_symbol_handle = unsafe { std::mem::transmute(rust) };
    assert_eq!(c, SG_NULL_HANDLE);
}
