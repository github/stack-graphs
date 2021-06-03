// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use crate::test_graphs;

#[test]
fn class_field_through_function_parameter() {
    let _ = test_graphs::class_field_through_function_parameter::new();
}

#[test]
fn cyclic_imports_python() {
    let _ = test_graphs::cyclic_imports_python::new();
}

#[test]
fn cyclic_imports_rust() {
    let _ = test_graphs::cyclic_imports_rust::new();
}

#[test]
fn sequenced_import_star() {
    let _ = test_graphs::sequenced_import_star::new();
}
