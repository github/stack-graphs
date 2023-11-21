// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2023, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use itertools::Itertools;
use pretty_assertions::assert_eq;

use stack_graphs::stats::*;

#[test]
fn empty_distribution() {
    let hist: FrequencyDistribution<i32> = FrequencyDistribution::default();

    assert_eq!(0, hist.unique());
    assert_eq!(0, hist.count());

    let result = hist.quantiles(0).into_iter().cloned().collect_vec();
    let expected: Vec<i32> = vec![];
    assert_eq!(expected, result);
}

#[test]
fn singleton_distribution() {
    let mut hist = FrequencyDistribution::default();
    hist.record(42);

    assert_eq!(1, hist.unique());
    assert_eq!(1, hist.count());

    let result = hist.quantiles(4).into_iter().cloned().collect_vec();
    let expected: Vec<i32> = vec![42, 42, 42, 42, 42];
    assert_eq!(expected, result);
}

#[test]
fn four_value_distribution() {
    let mut hist = FrequencyDistribution::default();
    hist.record(3);
    hist.record(4);
    hist.record(1);
    hist.record(2);

    assert_eq!(4, hist.unique());
    assert_eq!(4, hist.count());

    let result = hist.quantiles(4).into_iter().cloned().collect_vec();
    let expected: Vec<i32> = vec![1, 1, 2, 3, 4];
    assert_eq!(expected, result);
}
