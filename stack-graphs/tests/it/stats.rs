use itertools::Itertools;
use pretty_assertions::assert_eq;

use stack_graphs::stats::*;

#[test]
fn empty_distribution() {
    let hist: FrequencyDistribution<i32> = FrequencyDistribution::default();

    assert_eq!(0, hist.unique());
    assert_eq!(0, hist.total());

    let result = hist.quantiles(0).into_iter().cloned().collect_vec();
    let expected: Vec<i32> = vec![];
    assert_eq!(expected, result);
}

#[test]
fn singleton_distribution() {
    let mut hist = FrequencyDistribution::default();
    hist += 42;

    assert_eq!(1, hist.unique());
    assert_eq!(1, hist.total());

    let result = hist.quantiles(4).into_iter().cloned().collect_vec();
    let expected: Vec<i32> = vec![42, 42, 42, 42, 42];
    assert_eq!(expected, result);
}

#[test]
fn four_value_distribution() {
    let mut hist = FrequencyDistribution::default();
    hist += 3;
    hist += 4;
    hist += 1;
    hist += 2;

    assert_eq!(4, hist.unique());
    assert_eq!(4, hist.total());

    let result = hist.quantiles(4).into_iter().cloned().collect_vec();
    let expected: Vec<i32> = vec![1, 1, 2, 3, 4];
    assert_eq!(expected, result);
}
