// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
// ------------------------------------------------------------------------------------------------

use std::marker::PhantomData;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tree_sitter_graph::parse_error::TreeWithParseErrorVec;

use crate::CancellationFlag;

pub struct DisplayParseErrorsPretty<'a> {
    pub parse_errors: &'a TreeWithParseErrorVec,
    pub path: &'a Path,
    pub source: &'a str,
    pub max_errors: usize,
}

impl std::fmt::Display for DisplayParseErrorsPretty<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parse_errors = self.parse_errors.errors();
        for parse_error in parse_errors.iter().take(self.max_errors) {
            write!(f, "{}", parse_error.display_pretty(self.path, &self.source))?;
        }
        if parse_errors.len() > self.max_errors {
            let more_errors = parse_errors.len() - self.max_errors;
            write!(
                f,
                "{} more parse error{} omitted\n",
                more_errors,
                if more_errors > 1 { "s" } else { "" },
            )?;
        }
        Ok(())
    }
}

pub struct TreeSitterCancellationFlag<'a> {
    flag: Arc<AtomicUsize>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> TreeSitterCancellationFlag<'a> {
    fn from(cancellation_flag: &'a dyn CancellationFlag) -> Self {
        let flag = Arc::new(AtomicUsize::new(0));
        let thread_flag = Arc::downgrade(&flag);
        // The 'a lifetime on cancellation_flag is changed to a 'static lifetime
        // so that we can pass it to the polling thread. This is possible, because:
        //   (1) The lifetime parameter on `TreeSitterCancellationFlag` ensures it does
        //       not outlive the original reference.
        //   (2) The thread captures a weak reference to the flag, which ensures that
        //       `cancellation_flag` are only accessed as long as the flag exists.
        //   (3) The field `self.flag` is the only other reference to the flag, ensuring
        //       the flag does not outlive the struct.
        //   (4) The lifetime parameter `'a` ensures that the struct does not outlive the
        //       `cancellation_flag` reference.
        // All of this ensures that the thread will not access `cancellation_flag` beyond
        // its lifetime.
        let cancellation_flag: &'static dyn CancellationFlag =
            unsafe { std::mem::transmute(cancellation_flag) };
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(10));
                if let Some(flag) = thread_flag.upgrade() {
                    // the flag is still in use
                    if cancellation_flag.check("").is_err() {
                        // set flag and stop polling
                        flag.store(1, Ordering::Relaxed);
                        return;
                    }
                } else {
                    // the flag is not in use anymore, stop polling
                    return;
                }
            }
        });
        Self {
            flag,
            _phantom: PhantomData::default(),
        }
    }
}

impl<'a> AsRef<AtomicUsize> for TreeSitterCancellationFlag<'a> {
    fn as_ref(&self) -> &AtomicUsize {
        &self.flag
    }
}

impl<'a> From<&'a dyn CancellationFlag> for TreeSitterCancellationFlag<'a> {
    fn from(value: &'a dyn CancellationFlag) -> Self {
        Self::from(value)
    }
}
