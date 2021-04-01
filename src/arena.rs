// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Please see the COPYING file in this distribution for license details.
// ------------------------------------------------------------------------------------------------

//! Cache-friendly arena allocation for stack graph data.
//!
//! A stack graph is composed of instances of many different data types, and to store the graph
//! structure itself, we need cyclic or self-referential data types.  The typical way to achieve
//! this in Rust is to use [arena allocation][], where all of the instances of a particular type
//! are stored in a single vector.  You then use indexes into this vector to store references to a
//! data instance.  Because indexes are just numbers, you don't run afoul of borrow checker.  And
//! because all instances live together in a continguous region of memory, your data access
//! patterns are very cache-friendly.
//!
//! This module implements a simple arena allocation scheme for stack graphs.  An
//! [`Arena<T>`][`Arena`] is an arena that holds all of the instances of type `T` for a stack
//! graph.  A [`Handle<T>`][`Handle`] holds the index of a particular instance of `T` in its arena.
//! All of our stack graph data types then use handles to refer to other parts of the stack graph.
//!
//! Note that our arena implementation does not support deletion!  Any content that you add to a
//! [`StackGraph`][] will live as long as the stack graph itself does.  The entire region of memory
//! for each arena will be freed in a single operation when the stack graph is dropped.
//!
//! [arena allocation]: https://en.wikipedia.org/wiki/Region-based_memory_management
//! [`Arena`]: struct.Arena.html
//! [`Handle`]: struct.Handle.html
//! [`StackGraph`]: ../graph/struct.StackGraph.html

use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::num::NonZeroU32;

/// A handle to an instance of type `T` that was allocated from an [`Arena`][].
///
/// #### Safety
///
/// Because of the type parameter `T`, the compiler can ensure that you don't use a handle for one
/// type to index into an arena of another type.  However, if you have multiple arenas for the
/// _same type_, we do not do anything to ensure that you only use a handle with the corresponding
/// arena.
pub struct Handle<T> {
    index: NonZeroU32,
    _phantom: PhantomData<T>,
}

impl<T> Handle<T> {
    fn new(index: NonZeroU32) -> Handle<T> {
        Handle {
            index,
            _phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn as_usize(self) -> usize {
        self.index.get() as usize
    }
}

// Normally we would #[derive] all of these traits, but the auto-derived implementations all
// require that T implement the trait as well.  We don't store any real instances of T inside of
// Handle, so our implementations do _not_ require that.

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Handle<T> {
        Handle::new(self.index)
    }
}

impl<T> Copy for Handle<T> {}

impl<T> Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Handle")
            .field("index", &self.index)
            .finish()
    }
}

impl<T> Eq for Handle<T> {}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> Ord for Handle<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.index.partial_cmp(&other.index)
    }
}

/// Manages the life cycle of instances of type `T`.  You can allocate new instances of `T` from
/// the arena.  All of the instances managed by this arena will be dropped as a single operation
/// when the arena itself is dropped.
pub struct Arena<T> {
    items: Vec<T>,
}

impl<T> Arena<T> {
    /// Creates a new arena.
    pub fn new() -> Arena<T> {
        Arena { items: Vec::new() }
    }

    /// Adds a new instance to this arena, returning a stable handle to it.
    ///
    /// Note that we do not deduplicate instances of `T` in any way.  If you add two instances that
    /// have the same content, you will get distinct handles for each one.
    pub fn add(&mut self, item: T) -> Handle<T> {
        let index = self.items.len() as u32;
        self.items.push(item);
        Handle::new(unsafe { NonZeroU32::new_unchecked(index + 1) })
    }

    /// Dereferences a handle to an instance owned by this arena, returning a reference to it.
    pub fn get(&self, handle: Handle<T>) -> &T {
        &self.items[handle.as_usize() - 1]
    }
}
