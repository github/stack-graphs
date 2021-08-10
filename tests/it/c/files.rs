// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use libc::c_char;
use stack_graphs::arena::Handle;
use stack_graphs::c::sg_file_handle;
use stack_graphs::c::sg_files;
use stack_graphs::c::sg_stack_graph_add_files;
use stack_graphs::c::sg_stack_graph_files;
use stack_graphs::c::sg_stack_graph_free;
use stack_graphs::c::sg_stack_graph_new;
use stack_graphs::graph::File;

fn lengths(data: &[&'static str]) -> Vec<usize> {
    data.iter().map(|s| s.len()).collect()
}

fn get_file(arena: &sg_files, handle: Handle<File>) -> &str {
    let slice = unsafe { std::slice::from_raw_parts(arena.files, arena.count) };
    let file = &slice[handle.as_usize()];
    unsafe {
        let bytes = std::slice::from_raw_parts(file.name as *const u8, file.name_len);
        std::str::from_utf8_unchecked(bytes)
    }
}

#[test]
fn can_create_files() {
    let graph = sg_stack_graph_new();

    let filenames = ["a.py", "a.py", "b.py", "c.py"];
    let mut handles: [Option<Handle<File>>; 4] = [None; 4];
    sg_stack_graph_add_files(
        graph,
        filenames.len(),
        filenames.join("").as_ptr() as *const c_char,
        lengths(&filenames).as_ptr(),
        handles.as_mut_ptr() as *mut sg_file_handle,
    );

    // All of the files should have been created successfully
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

    // We should be able to dereference into the files arena to get the file content.
    let file_arena = sg_stack_graph_files(graph);
    assert_eq!(get_file(&file_arena, a1), "a.py");
    assert_eq!(get_file(&file_arena, a2), "a.py");
    assert_eq!(get_file(&file_arena, b), "b.py");
    assert_eq!(get_file(&file_arena, c), "c.py");

    sg_stack_graph_free(graph);
}
