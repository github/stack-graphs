// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use controlled_option::ControlledOption;
use libc::c_char;
use stack_graphs::arena::Handle;
use stack_graphs::c::sg_stack_graph_add_strings;
use stack_graphs::c::sg_stack_graph_free;
use stack_graphs::c::sg_stack_graph_new;
use stack_graphs::c::sg_stack_graph_strings;
use stack_graphs::c::sg_string_handle;
use stack_graphs::c::sg_strings;
use stack_graphs::c::SG_NULL_HANDLE;
use stack_graphs::graph::InternedString;

fn lengths(data: &[&'static str]) -> Vec<usize> {
    data.iter().map(|s| s.len()).collect()
}

fn get_string(arena: &sg_strings, handle: Handle<InternedString>) -> &str {
    let slice = unsafe { std::slice::from_raw_parts(arena.strings, arena.count) };
    let string = &slice[handle.as_usize()];
    unsafe {
        let bytes = std::slice::from_raw_parts(string.content as *const u8, string.length);
        std::str::from_utf8_unchecked(bytes)
    }
}

#[test]
fn can_create_strings() {
    let graph = sg_stack_graph_new();

    let string_data = ["a", "a", "b", "c"];
    let mut handles: [Option<Handle<InternedString>>; 4] = [None; 4];
    sg_stack_graph_add_strings(
        graph,
        string_data.len(),
        string_data.join("").as_ptr() as *const c_char,
        lengths(&string_data).as_ptr(),
        handles.as_mut_ptr() as *mut sg_string_handle,
    );

    // All of the strings should have been created successfully
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

    // We should be able to dereference into the strings arena to get the string content.
    let string_arena = sg_stack_graph_strings(graph);
    assert_eq!(get_string(&string_arena, a1), "a");
    assert_eq!(get_string(&string_arena, a2), "a");
    assert_eq!(get_string(&string_arena, b), "b");
    assert_eq!(get_string(&string_arena, c), "c");

    sg_stack_graph_free(graph);
}

#[test]
#[allow(unused_assignments)]
fn verify_null_string_representation() {
    let bytes = [0x55u8; std::mem::size_of::<Handle<InternedString>>()];
    let mut rust: ControlledOption<Handle<InternedString>> = unsafe { std::mem::transmute(bytes) };
    rust = ControlledOption::none();
    let c: sg_string_handle = unsafe { std::mem::transmute(rust) };
    assert_eq!(c, SG_NULL_HANDLE);
}
