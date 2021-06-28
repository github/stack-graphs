// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

pub(crate) fn equals_option<A, B, F>(a: Option<A>, b: Option<B>, mut eq: F) -> bool
where
    F: FnMut(A, B) -> bool,
{
    match a {
        Some(a) => match b {
            Some(b) => eq(a, b),
            None => false,
        },
        None => match b {
            Some(_) => false,
            None => true,
        },
    }
}

pub(crate) fn cmp_option<T, F>(a: Option<T>, b: Option<T>, mut cmp: F) -> std::cmp::Ordering
where
    F: FnMut(T, T) -> std::cmp::Ordering,
{
    use std::cmp::Ordering;
    match a {
        Some(a) => match b {
            Some(b) => cmp(a, b),
            None => Ordering::Greater,
        },
        None => match b {
            Some(_) => Ordering::Less,
            None => Ordering::Equal,
        },
    }
}
