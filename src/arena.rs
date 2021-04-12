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
use std::ops::Index;
use std::ops::IndexMut;

//-------------------------------------------------------------------------------------------------
// Arenas and handles

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
    fn as_usize(self) -> usize {
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

//-------------------------------------------------------------------------------------------------
// Supplemental arenas

/// A supplemental arena lets you store additional data about some data type that is itself stored
/// in an [`Arena`][].
///
/// We implement `Index` and `IndexMut` for a more ergonomic syntax.  Please note that when
/// indexing in an _immutable_ context, we **_panic_** if you try to access data for a handle that
/// doesn't exist in the arena.  (Use the [`get`][] method if you don't know whether the value
/// exists or not.)  In a _mutable_ context, we automatically create a `Default` instance of the
/// type if there isn't already an instance for that handle in the arena.
///
/// ```
/// # use stack_graphs::arena::Arena;
/// # use stack_graphs::arena::SupplementalArena;
/// // We need an Arena to create handles.
/// let mut arena = Arena::<u32>::new();
/// let handle = arena.add(1);
///
/// let mut supplemental = SupplementalArena::<u32, String>::new();
///
/// // But indexing will panic if the element doesn't already exist.
/// // assert_eq!(supplemental[handle].as_str(), "");
///
/// // The `get` method is always safe, since it returns an Option.
/// assert_eq!(supplemental.get(handle), None);
///
/// // Once we've added the element to the supplemental arena, indexing
/// // won't panic anymore.
/// supplemental[handle] = "hello".to_string();
/// assert_eq!(supplemental[handle].as_str(), "hello");
/// ```
///
/// [`Arena`]: struct.Arena.html
/// [`get`]: #method.get
pub struct SupplementalArena<H, T> {
    items: Vec<T>,
    _phantom: PhantomData<H>,
}

impl<H, T> SupplementalArena<H, T> {
    /// Creates a new, empty supplemental arena.
    pub fn new() -> SupplementalArena<H, T> {
        SupplementalArena {
            items: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Creates a new, empty supplemental arena, preallocating enough space to store supplemental
    /// data for all of the instances that have already been allocated in a (regular) arena.
    pub fn with_capacity(arena: &Arena<H>) -> SupplementalArena<H, T> {
        SupplementalArena {
            items: Vec::with_capacity(arena.items.len()),
            _phantom: PhantomData,
        }
    }

    /// Returns the item belonging to a particular handle, if it exists.
    pub fn get(&self, handle: Handle<H>) -> Option<&T> {
        self.items.get(handle.as_usize() - 1)
    }

    /// Returns a mutable reference to the item belonging to a particular handle, if it exists.
    pub fn get_mut(&mut self, handle: Handle<H>) -> Option<&mut T> {
        self.items.get_mut(handle.as_usize() - 1)
    }
}

impl<H, T> SupplementalArena<H, T>
where
    T: Default,
{
    /// Returns a mutable reference to the item belonging to a particular handle, creating it first
    /// (using the type's `Default` implementation) if it doesn't already exist.
    pub fn get_mut_or_default(&mut self, handle: Handle<H>) -> &mut T {
        let index = handle.as_usize();
        if self.items.len() < index {
            self.items.resize_with(index, || T::default());
        }
        unsafe { self.items.get_unchecked_mut(index - 1) }
    }
}

impl<H, T> Default for SupplementalArena<H, T> {
    fn default() -> SupplementalArena<H, T> {
        SupplementalArena::new()
    }
}

impl<H, T> Index<Handle<H>> for SupplementalArena<H, T> {
    type Output = T;
    fn index(&self, handle: Handle<H>) -> &T {
        &self.items[handle.as_usize() - 1]
    }
}

impl<H, T> IndexMut<Handle<H>> for SupplementalArena<H, T>
where
    T: Default,
{
    fn index_mut(&mut self, handle: Handle<H>) -> &mut T {
        self.get_mut_or_default(handle)
    }
}
