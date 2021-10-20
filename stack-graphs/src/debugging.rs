// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

#[cfg(feature = "copious-debugging")]
#[macro_export]
macro_rules! copious_debugging {
    ($($arg:tt)*) => {{ ::std::eprintln!($($arg)*); }}

}

#[cfg(not(feature = "copious-debugging"))]
#[macro_export]
macro_rules! copious_debugging {
    ($($arg:tt)*) => {};
}
