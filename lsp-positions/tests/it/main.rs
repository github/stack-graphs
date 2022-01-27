// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use unicode_segmentation::UnicodeSegmentation as _;

use lsp_positions::Offset;

fn check_offsets(line: &str) {
    let offsets = Offset::all_chars(line).collect::<Vec<_>>();
    assert!(!offsets.is_empty());

    assert_eq!(offsets.first().unwrap().utf8_offset, 0);
    assert_eq!(offsets.last().unwrap().utf8_offset, line.len());

    assert_eq!(offsets.first().unwrap().utf16_offset, 0);
    assert_eq!(
        offsets.last().unwrap().utf16_offset,
        line.encode_utf16().count()
    );

    assert_eq!(offsets.first().unwrap().grapheme_offset, 0);
    assert_eq!(
        offsets.last().unwrap().grapheme_offset,
        line.graphemes(true).count()
    );

    for (index, (utf8_offset, _)) in line.char_indices().enumerate() {
        assert_eq!(offsets[index].utf8_offset, utf8_offset);

        let prefix = &line[0..utf8_offset];
        let utf16_offset = prefix.encode_utf16().count();
        assert_eq!(offsets[index].utf16_offset, utf16_offset);

        let prefix = &line[0..utf8_offset];
        let grapheme_offset = prefix.graphemes(true).count();
        assert_eq!(offsets[index].grapheme_offset, grapheme_offset);
    }
}

#[test]
fn can_calculate_column_offsets() {
    check_offsets("from a import *");
    check_offsets("print '❤️', b, '👨‍👨‍👧', c");
    check_offsets("print '✨✨✨', d");
}
