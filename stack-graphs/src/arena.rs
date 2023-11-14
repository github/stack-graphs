// -*- coding: utf-8 -*-
// ------------------------------------------------------------------------------------------------
// Copyright © 2021, stack-graphs authors.
// Licensed under either of Apache License, Version 2.0, or MIT license, at your option.
// Please see the LICENSE-APACHE or LICENSE-MIT files in this distribution for license details.
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

use std::cell::Cell;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::num::NonZeroU32;
use std::ops::Index;
use std::ops::IndexMut;

use bitvec::vec::BitVec;
use controlled_option::Niche;

use crate::utils::cmp_option;
use crate::utils::equals_option;

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
#[repr(transparent)]
pub struct Handle<T> {
    index: NonZeroU32,
    _phantom: PhantomData<T>,
}

impl<T> Handle<T> {
    pub(crate) fn new(index: NonZeroU32) -> Handle<T> {
        Handle {
            index,
            _phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn as_u32(self) -> u32 {
        self.index.get()
    }

    #[inline(always)]
    pub fn as_usize(self) -> usize {
        self.index.get() as usize
    }
}

impl<T> Niche for Handle<T> {
    type Output = u32;

    #[inline]
    fn none() -> Self::Output {
        0
    }

    #[inline]
    fn is_none(value: &Self::Output) -> bool {
        *value == 0
    }

    #[inline]
    fn into_some(value: Self) -> Self::Output {
        value.index.get()
    }

    #[inline]
    fn from_some(value: Self::Output) -> Self {
        Self::new(unsafe { NonZeroU32::new_unchecked(value) })
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

// Handles are always Send and Sync, even if the underlying types are not.  After all, a handle is
// just a number!  And you _also_ need access to the Arena (which _won't_ be Send/Sync if T isn't)
// to dereference the handle.
unsafe impl<T> Send for Handle<T> {}
unsafe impl<T> Sync for Handle<T> {}

/// Manages the life cycle of instances of type `T`.  You can allocate new instances of `T` from
/// the arena.  All of the instances managed by this arena will be dropped as a single operation
/// when the arena itself is dropped.
pub struct Arena<T> {
    items: Vec<MaybeUninit<T>>,
}

impl<T> Drop for Arena<T> {
    fn drop(&mut self) {
        unsafe {
            let items = std::mem::transmute::<_, &mut [T]>(&mut self.items[1..]) as *mut [T];
            items.drop_in_place();
        }
    }
}

impl<T> Arena<T> {
    /// Creates a new arena.
    pub fn new() -> Arena<T> {
        Arena {
            items: vec![MaybeUninit::uninit()],
        }
    }

    /// Clear the arena, keeping underlying allocated capacity.  After this, all previous handles into
    /// the arena are invalid.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.items.truncate(1);
    }

    /// Adds a new instance to this arena, returning a stable handle to it.
    ///
    /// Note that we do not deduplicate instances of `T` in any way.  If you add two instances that
    /// have the same content, you will get distinct handles for each one.
    pub fn add(&mut self, item: T) -> Handle<T> {
        let index = self.items.len() as u32;
        self.items.push(MaybeUninit::new(item));
        Handle::new(unsafe { NonZeroU32::new_unchecked(index) })
    }

    /// Dereferences a handle to an instance owned by this arena, returning a reference to it.
    pub fn get(&self, handle: Handle<T>) -> &T {
        unsafe { std::mem::transmute(&self.items[handle.as_usize()]) }
    }
    ///
    /// Dereferences a handle to an instance owned by this arena, returning a mutable reference to
    /// it.
    pub fn get_mut(&mut self, handle: Handle<T>) -> &mut T {
        unsafe { std::mem::transmute(&mut self.items[handle.as_usize()]) }
    }

    /// Returns an iterator of all of the handles in this arena.  (Note that this iterator does not
    /// retain a reference to the arena!)
    pub fn iter_handles(&self) -> impl Iterator<Item = Handle<T>> {
        (1..self.items.len())
            .into_iter()
            .map(|index| Handle::new(unsafe { NonZeroU32::new_unchecked(index as u32) }))
    }

    /// Returns a pointer to this arena's storage.
    pub(crate) fn as_ptr(&self) -> *const T {
        self.items.as_ptr() as *const T
    }

    /// Returns the number of instances stored in this arena.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.items.len()
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
    items: Vec<MaybeUninit<T>>,
    _phantom: PhantomData<H>,
}

impl<H, T> Drop for SupplementalArena<H, T> {
    fn drop(&mut self) {
        unsafe {
            let items = std::mem::transmute::<_, &mut [T]>(&mut self.items[1..]) as *mut [T];
            items.drop_in_place();
        }
    }
}

impl<H, T> SupplementalArena<H, T> {
    /// Creates a new, empty supplemental arena.
    pub fn new() -> SupplementalArena<H, T> {
        SupplementalArena {
            items: vec![MaybeUninit::uninit()],
            _phantom: PhantomData,
        }
    }

    /// Clear the supplemantal arena, keeping underlying allocated capacity.  After this,
    /// all previous handles into the arena are invalid.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.items.truncate(1);
    }

    /// Creates a new, empty supplemental arena, preallocating enough space to store supplemental
    /// data for all of the instances that have already been allocated in a (regular) arena.
    pub fn with_capacity(arena: &Arena<H>) -> SupplementalArena<H, T> {
        let mut items = Vec::with_capacity(arena.items.len());
        items[0] = MaybeUninit::uninit();
        SupplementalArena {
            items,
            _phantom: PhantomData,
        }
    }

    /// Returns the item belonging to a particular handle, if it exists.
    pub fn get(&self, handle: Handle<H>) -> Option<&T> {
        self.items
            .get(handle.as_usize())
            .map(|x| unsafe { &*(x.as_ptr()) })
    }

    /// Returns a mutable reference to the item belonging to a particular handle, if it exists.
    pub fn get_mut(&mut self, handle: Handle<H>) -> Option<&mut T> {
        self.items
            .get_mut(handle.as_usize())
            .map(|x| unsafe { &mut *(x.as_mut_ptr()) })
    }

    /// Returns a pointer to this arena's storage.
    pub(crate) fn as_ptr(&self) -> *const T {
        self.items.as_ptr() as *const T
    }

    /// Returns the number of instances stored in this arena.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Iterate over the items in this arena.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.items
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, x)| (Handle::from_some(i as u32), unsafe { &*(x.as_ptr()) }))
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
        if self.items.len() <= index {
            self.items
                .resize_with(index + 1, || MaybeUninit::new(T::default()));
        }
        unsafe { std::mem::transmute(&mut self.items[handle.as_usize()]) }
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
        unsafe { std::mem::transmute(&self.items[handle.as_usize()]) }
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

//-------------------------------------------------------------------------------------------------
// Handle sets

/// Contains a set of handles, encoded efficiently using a bit set.
#[repr(C)]
pub struct HandleSet<T> {
    elements: BitVec<u32, bitvec::order::Lsb0>,
    _phantom: PhantomData<T>,
}

impl<T> HandleSet<T> {
    /// Creates a new, empty handle set.
    pub fn new() -> HandleSet<T> {
        HandleSet::default()
    }

    /// Removes all elements from this handle set.
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Returns whether this set contains a particular handle.
    pub fn contains(&self, handle: Handle<T>) -> bool {
        let index = handle.as_usize();
        self.elements.get(index).map(|bit| *bit).unwrap_or(false)
    }

    /// Adds a handle to this set.
    pub fn add(&mut self, handle: Handle<T>) {
        let index = handle.as_usize();
        if self.elements.len() <= index {
            self.elements.resize(index + 1, false);
        }
        let mut bit = unsafe { self.elements.get_unchecked_mut(index) };
        *bit = true;
    }

    /// Removes a handle from this set.
    pub fn remove(&mut self, handle: Handle<T>) {
        let index = handle.as_usize();
        if let Some(mut bit) = self.elements.get_mut(index) {
            *bit = false;
        }
    }

    /// Returns an iterator of all of the handles in this set.
    pub fn iter(&self) -> impl Iterator<Item = Handle<T>> + '_ {
        self.elements
            .iter_ones()
            .map(|index| Handle::new(unsafe { NonZeroU32::new_unchecked(index as u32) }))
    }

    /// Returns a pointer to this set's storage.
    pub(crate) fn as_ptr(&self) -> *const u32 {
        self.elements.as_bitptr().pointer()
    }

    /// Returns the number of instances stored in this arena.
    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.elements.as_raw_slice().len()
    }
}

impl<T> Default for HandleSet<T> {
    fn default() -> HandleSet<T> {
        HandleSet {
            elements: BitVec::default(),
            _phantom: PhantomData,
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Arena-allocated lists

/// An arena-allocated singly-linked list.
///
/// Linked lists are often a poor choice because they aren't very cache-friendly.  However, this
/// linked list implementation _should_ be cache-friendly, since the individual cells are allocated
/// out of an arena.
#[repr(C)]
#[derive(Niche)]
pub struct List<T> {
    // The value of this handle will be EMPTY_LIST_HANDLE if the list is empty.  For an
    // Option<List<T>>, the value will be zero (via the Option<NonZero> optimization) if the list
    // is None.
    #[niche]
    cells: Handle<ListCell<T>>,
}

#[doc(hidden)]
#[repr(C)]
pub struct ListCell<T> {
    head: T,
    // The value of this handle will be EMPTY_LIST_HANDLE if this is the last element of the list.
    tail: Handle<ListCell<T>>,
}

const EMPTY_LIST_HANDLE: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(u32::MAX) };

// An arena that's used to manage `List<T>` instances.
//
// (Note that the arena doesn't store `List<T>` itself; it stores the `ListCell<T>`s that the lists
// are made of.)
pub type ListArena<T> = Arena<ListCell<T>>;

impl<T> List<T> {
    /// Creates a new `ListArena` that will manage lists of this type.
    pub fn new_arena() -> ListArena<T> {
        ListArena::new()
    }

    /// Returns whether this list is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.cells.index == EMPTY_LIST_HANDLE
    }

    /// Returns an empty list.
    pub fn empty() -> List<T> {
        List {
            cells: Handle::new(EMPTY_LIST_HANDLE),
        }
    }

    pub fn from_handle(handle: Handle<ListCell<T>>) -> List<T> {
        List { cells: handle }
    }

    /// Returns a handle to the head of the list.
    pub fn handle(&self) -> Handle<ListCell<T>> {
        self.cells
    }

    /// Pushes a new element onto the front of this list.
    pub fn push_front(&mut self, arena: &mut ListArena<T>, head: T) {
        self.cells = arena.add(ListCell {
            head,
            tail: self.cells,
        });
    }

    /// Removes and returns the element at the front of this list.  If the list is empty, returns
    /// `None`.
    pub fn pop_front<'a>(&mut self, arena: &'a ListArena<T>) -> Option<&'a T> {
        if self.is_empty() {
            return None;
        }
        let cell = arena.get(self.cells);
        self.cells = cell.tail;
        Some(&cell.head)
    }

    /// Returns an iterator over the elements of this list.
    pub fn iter<'a>(mut self, arena: &'a ListArena<T>) -> impl Iterator<Item = &'a T> + 'a {
        std::iter::from_fn(move || self.pop_front(arena))
    }
}

impl<T> List<T> {
    pub fn equals_with<F>(mut self, arena: &ListArena<T>, mut other: List<T>, mut eq: F) -> bool
    where
        F: FnMut(&T, &T) -> bool,
    {
        loop {
            if self.cells == other.cells {
                return true;
            }
            if !equals_option(self.pop_front(arena), other.pop_front(arena), &mut eq) {
                return false;
            }
        }
    }

    pub fn cmp_with<F>(
        mut self,
        arena: &ListArena<T>,
        mut other: List<T>,
        mut cmp: F,
    ) -> std::cmp::Ordering
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        use std::cmp::Ordering;
        loop {
            if self.cells == other.cells {
                return Ordering::Equal;
            }
            match cmp_option(self.pop_front(arena), other.pop_front(arena), &mut cmp) {
                Ordering::Equal => (),
                result @ _ => return result,
            }
        }
    }
}

impl<T> List<T>
where
    T: Eq,
{
    pub fn equals(self, arena: &ListArena<T>, other: List<T>) -> bool {
        self.equals_with(arena, other, |a, b| *a == *b)
    }
}

impl<T> List<T>
where
    T: Ord,
{
    pub fn cmp(self, arena: &ListArena<T>, other: List<T>) -> std::cmp::Ordering {
        self.cmp_with(arena, other, |a, b| a.cmp(b))
    }
}

// Normally we would #[derive] all of these traits, but the auto-derived implementations all
// require that T implement the trait as well.  We don't store any real instances of T inside of
// List, so our implementations do _not_ require that.

impl<T> Clone for List<T> {
    fn clone(&self) -> List<T> {
        List { cells: self.cells }
    }
}

impl<T> Copy for List<T> {}

//-------------------------------------------------------------------------------------------------
// Reversible arena-allocated list

/// An arena-allocated list that can be reversed.
///
/// Well, that is, you can reverse a [`List`][] just fine by yourself.  This type takes care of
/// doing that for you, and importantly, _saves the result_ so that if you only have to compute the
/// reversal once even if you need to access it multiple times.
///
/// [`List`]: struct.List.html
#[repr(C)]
#[derive(Niche)]
pub struct ReversibleList<T> {
    #[niche]
    cells: Handle<ReversibleListCell<T>>,
}

#[repr(C)]
#[doc(hidden)]
pub struct ReversibleListCell<T> {
    head: T,
    tail: Handle<ReversibleListCell<T>>,
    reversed: Cell<Option<Handle<ReversibleListCell<T>>>>,
}

// An arena that's used to manage `ReversibleList<T>` instances.
//
// (Note that the arena doesn't store `ReversibleList<T>` itself; it stores the
// `ReversibleListCell<T>`s that the lists are made of.)
pub type ReversibleListArena<T> = Arena<ReversibleListCell<T>>;

impl<T> ReversibleList<T> {
    /// Creates a new `ReversibleListArena` that will manage lists of this type.
    pub fn new_arena() -> ReversibleListArena<T> {
        ReversibleListArena::new()
    }

    /// Returns whether this list is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        ReversibleListCell::is_empty_handle(self.cells)
    }

    /// Returns an empty list.
    pub fn empty() -> ReversibleList<T> {
        ReversibleList {
            cells: ReversibleListCell::empty_handle(),
        }
    }

    /// Returns whether we have already calculated the reversal of this list.
    pub fn have_reversal(&self, arena: &ReversibleListArena<T>) -> bool {
        if self.is_empty() {
            // The empty list is already reversed.
            return true;
        }
        arena.get(self.cells).reversed.get().is_some()
    }

    /// Pushes a new element onto the front of this list.
    pub fn push_front(&mut self, arena: &mut ReversibleListArena<T>, head: T) {
        self.cells = arena.add(ReversibleListCell::new(head, self.cells, None));
    }

    /// Removes and returns the element at the front of this list.  If the list is empty, returns
    /// `None`.
    pub fn pop_front<'a>(&mut self, arena: &'a ReversibleListArena<T>) -> Option<&'a T> {
        if self.is_empty() {
            return None;
        }
        let cell = arena.get(self.cells);
        self.cells = cell.tail;
        Some(&cell.head)
    }

    /// Returns an iterator over the elements of this list.
    pub fn iter<'a>(
        mut self,
        arena: &'a ReversibleListArena<T>,
    ) -> impl Iterator<Item = &'a T> + 'a {
        std::iter::from_fn(move || self.pop_front(arena))
    }
}

impl<T> ReversibleList<T>
where
    T: Clone,
{
    /// Reverses the list.  Since we're already caching everything in an arena, we make sure to
    /// only calculate the reversal once, returning it as-is if you call this function multiple
    /// times.
    pub fn reverse(&mut self, arena: &mut ReversibleListArena<T>) {
        if self.is_empty() {
            return;
        }
        self.ensure_reversal_available(arena);
        self.cells = arena.get(self.cells).reversed.get().unwrap();
    }

    /// Ensures that the reversal of this list is available.  It can be useful to precalculate this
    /// when you have mutable access to the arena, so that you can then reverse and un-reverse the
    /// list later when you only have shared access to it.
    pub fn ensure_reversal_available(&mut self, arena: &mut ReversibleListArena<T>) {
        // First check to see if the list has already been reversed.
        if self.is_empty() {
            // The empty list is already reversed.
            return;
        }
        if arena.get(self.cells).reversed.get().is_some() {
            return;
        }

        // If not, reverse the list and cache the result.
        let new_reversed = ReversibleListCell::reverse(self.cells, arena);
        arena.get(self.cells).reversed.set(Some(new_reversed));
    }
}

impl<T> ReversibleList<T> {
    /// Reverses the list, assuming that the reversal has already been computed.  If it hasn't we
    /// return an error.
    pub fn reverse_reused(&mut self, arena: &ReversibleListArena<T>) -> Result<(), ()> {
        if self.is_empty() {
            // The empty list is already reversed.
            return Ok(());
        }
        self.cells = arena.get(self.cells).reversed.get().ok_or(())?;
        Ok(())
    }
}

impl<T> ReversibleListCell<T> {
    fn new(
        head: T,
        tail: Handle<ReversibleListCell<T>>,
        reversed: Option<Handle<ReversibleListCell<T>>>,
    ) -> ReversibleListCell<T> {
        ReversibleListCell {
            head,
            tail,
            reversed: Cell::new(reversed),
        }
    }

    fn empty_handle() -> Handle<ReversibleListCell<T>> {
        Handle::new(EMPTY_LIST_HANDLE)
    }

    fn is_empty_handle(handle: Handle<ReversibleListCell<T>>) -> bool {
        handle.index == EMPTY_LIST_HANDLE
    }
}

impl<T> ReversibleListCell<T>
where
    T: Clone,
{
    fn reverse(
        forwards: Handle<ReversibleListCell<T>>,
        arena: &mut ReversibleListArena<T>,
    ) -> Handle<ReversibleListCell<T>> {
        let mut reversed = ReversibleListCell::empty_handle();
        let mut current = forwards;
        while !ReversibleListCell::is_empty_handle(current) {
            let cell = arena.get(current);
            let head = cell.head.clone();
            current = cell.tail;
            reversed = arena.add(Self::new(
                head,
                reversed,
                // The reversal of the reversal that we just calculated is our original list!  Go
                // ahead and cache that away preemptively.
                if ReversibleListCell::is_empty_handle(current) {
                    Some(forwards)
                } else {
                    None
                },
            ));
        }
        reversed
    }
}

impl<T> ReversibleList<T> {
    pub fn equals_with<F>(
        mut self,
        arena: &ReversibleListArena<T>,
        mut other: ReversibleList<T>,
        mut eq: F,
    ) -> bool
    where
        F: FnMut(&T, &T) -> bool,
    {
        loop {
            if self.cells == other.cells {
                return true;
            }
            if !equals_option(self.pop_front(arena), other.pop_front(arena), &mut eq) {
                return false;
            }
        }
    }

    pub fn cmp_with<F>(
        mut self,
        arena: &ReversibleListArena<T>,
        mut other: ReversibleList<T>,
        mut cmp: F,
    ) -> std::cmp::Ordering
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        use std::cmp::Ordering;
        loop {
            if self.cells == other.cells {
                return Ordering::Equal;
            }
            match cmp_option(self.pop_front(arena), other.pop_front(arena), &mut cmp) {
                Ordering::Equal => (),
                result @ _ => return result,
            }
        }
    }
}

impl<T> ReversibleList<T>
where
    T: Eq,
{
    pub fn equals(self, arena: &ReversibleListArena<T>, other: ReversibleList<T>) -> bool {
        self.equals_with(arena, other, |a, b| *a == *b)
    }
}

impl<T> ReversibleList<T>
where
    T: Ord,
{
    pub fn cmp(
        self,
        arena: &ReversibleListArena<T>,
        other: ReversibleList<T>,
    ) -> std::cmp::Ordering {
        self.cmp_with(arena, other, |a, b| a.cmp(b))
    }
}

// Normally we would #[derive] all of these traits, but the auto-derived implementations all
// require that T implement the trait as well.  We don't store any real instances of T inside of
// ReversibleList, so our implementations do _not_ require that.

impl<T> Clone for ReversibleList<T> {
    fn clone(&self) -> ReversibleList<T> {
        ReversibleList { cells: self.cells }
    }
}

impl<T> Copy for ReversibleList<T> {}

//-------------------------------------------------------------------------------------------------
// Arena-allocated deque

/// An arena-allocated deque.
///
/// Under the covers, this is implemented as a [`List`][].  Because these lists are singly-linked,
/// we can only add elements to, and remove them from, one side of the list.
///
/// To handle this, each deque stores its contents either _forwards_ or _backwards_.  We
/// automatically shift between these two representations as needed, depending on the requirements
/// of each method.
///
/// Note that we cache the result of reversing the list, so it should be quick to switch back and
/// forth between representations _as long as you have not added any additional elements to the
/// deque_!  If performance is critical, you should ensure that you don't call methods in a pattern
/// that causes the deque to reverse itself each time you add an element.
///
/// [`List`]: struct.List.html
#[repr(C)]
#[derive(Niche)]
pub struct Deque<T> {
    #[niche]
    list: ReversibleList<T>,
    direction: DequeDirection,
}

#[repr(C)]
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum DequeDirection {
    Forwards,
    Backwards,
}

impl std::ops::Not for DequeDirection {
    type Output = DequeDirection;
    fn not(self) -> DequeDirection {
        match self {
            DequeDirection::Forwards => DequeDirection::Backwards,
            DequeDirection::Backwards => DequeDirection::Forwards,
        }
    }
}

// An arena that's used to manage `Deque<T>` instances.
pub type DequeArena<T> = ReversibleListArena<T>;

impl<T> Deque<T> {
    /// Creates a new `DequeArena` that will manage deques of this type.
    pub fn new_arena() -> DequeArena<T> {
        ReversibleList::new_arena()
    }

    /// Returns whether this deque is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Returns an empty deque.
    pub fn empty() -> Deque<T> {
        Deque {
            list: ReversibleList::empty(),
            // A philosophical question for you: is the empty list forwards or backwards?  It
            // doesn't really matter which one we choose here; if we immediately start pushing onto
            // the back, we'll "reverse" the current list before proceeding, but reversing the
            // empty list is a no-op.
            direction: DequeDirection::Forwards,
        }
    }

    /// Returns whether we have already calculated the reversal of this deque.
    pub fn have_reversal(&self, arena: &DequeArena<T>) -> bool {
        self.list.have_reversal(arena)
    }

    fn is_backwards(&self) -> bool {
        matches!(self.direction, DequeDirection::Backwards)
    }

    fn is_forwards(&self) -> bool {
        matches!(self.direction, DequeDirection::Forwards)
    }

    /// Returns an iterator over the contents of this deque, with no guarantee about the ordering of
    /// the elements.  (By not caring about the ordering of the elements, you can call this method
    /// regardless of which direction the deque's elements are currently stored.  And that, in
    /// turn, means that we only need shared access to the arena, and not mutable access to it.)
    pub fn iter_unordered<'a>(&self, arena: &'a DequeArena<T>) -> impl Iterator<Item = &'a T> + 'a {
        self.list.iter(arena)
    }
}

impl<T> Deque<T>
where
    T: Clone,
{
    /// Ensures that this deque has computed its backwards-facing list of elements.
    pub fn ensure_backwards(&mut self, arena: &mut DequeArena<T>) {
        if self.is_backwards() {
            return;
        }
        self.list.reverse(arena);
        self.direction = DequeDirection::Backwards;
    }

    /// Ensures that this deque has computed its forwards-facing list of elements.
    pub fn ensure_forwards(&mut self, arena: &mut DequeArena<T>) {
        if self.is_forwards() {
            return;
        }
        self.list.reverse(arena);
        self.direction = DequeDirection::Forwards;
    }

    /// Pushes a new element onto the front of this deque.
    pub fn push_front(&mut self, arena: &mut DequeArena<T>, element: T) {
        self.ensure_forwards(arena);
        self.list.push_front(arena, element);
    }

    /// Pushes a new element onto the back of this deque.
    pub fn push_back(&mut self, arena: &mut DequeArena<T>, element: T) {
        self.ensure_backwards(arena);
        self.list.push_front(arena, element);
    }

    /// Removes and returns the element from the front of this deque.  If the deque is empty,
    /// returns `None`.
    pub fn pop_front<'a>(&mut self, arena: &'a mut DequeArena<T>) -> Option<&'a T> {
        self.ensure_forwards(arena);
        self.list.pop_front(arena)
    }

    /// Removes and returns the element from the back of this deque.  If the deque is empty,
    /// returns `None`.
    pub fn pop_back<'a>(&mut self, arena: &'a mut DequeArena<T>) -> Option<&'a T> {
        self.ensure_backwards(arena);
        self.list.pop_front(arena)
    }

    /// Returns an iterator over the contents of this deque in a forwards direction.
    pub fn iter<'a>(&self, arena: &'a mut DequeArena<T>) -> impl Iterator<Item = &'a T> + 'a {
        let mut list = self.list;
        if self.is_backwards() {
            list.reverse(arena);
        }
        list.iter(arena)
    }

    /// Returns an iterator over the contents of this deque in a backwards direction.
    pub fn iter_reversed<'a>(
        &self,
        arena: &'a mut DequeArena<T>,
    ) -> impl Iterator<Item = &'a T> + 'a {
        let mut list = self.list;
        if self.is_forwards() {
            list.reverse(arena);
        }
        list.iter(arena)
    }

    /// Ensures that both deques are stored in the same direction.  It doesn't matter _which_
    /// direction, as long as they're the same, so do the minimum amount of work to bring this
    /// about.  (In particular, if we've already calculated the reversal of one of the deques,
    /// reverse that one.)
    fn ensure_same_direction(&mut self, arena: &mut DequeArena<T>, other: &mut Deque<T>) {
        if self.direction == other.direction {
            return;
        }
        if self.list.have_reversal(arena) {
            self.list.reverse(arena);
            self.direction = !self.direction;
        } else {
            other.list.reverse(arena);
            other.direction = !other.direction;
        }
    }
}

impl<T> Deque<T>
where
    T: Clone,
{
    pub fn equals_with<F>(mut self, arena: &mut DequeArena<T>, mut other: Deque<T>, eq: F) -> bool
    where
        F: FnMut(&T, &T) -> bool,
    {
        self.ensure_same_direction(arena, &mut other);
        self.list.equals_with(arena, other.list, eq)
    }

    pub fn cmp_with<F>(
        mut self,
        arena: &mut DequeArena<T>,
        mut other: Deque<T>,
        cmp: F,
    ) -> std::cmp::Ordering
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        // To compare, we need boths deques to specifically be pointing forwards, and not just in
        // the same direction, so that we get the lexicographic comparison correct.
        self.ensure_forwards(arena);
        other.ensure_forwards(arena);
        self.list.cmp_with(arena, other.list, cmp)
    }
}

impl<T> Deque<T>
where
    T: Clone + Eq,
{
    pub fn equals(self, arena: &mut DequeArena<T>, other: Deque<T>) -> bool {
        self.equals_with(arena, other, |a, b| *a == *b)
    }
}

impl<T> Deque<T>
where
    T: Clone + Ord,
{
    pub fn cmp(self, arena: &mut DequeArena<T>, other: Deque<T>) -> std::cmp::Ordering {
        self.cmp_with(arena, other, |a, b| a.cmp(b))
    }
}

impl<T> Deque<T> {
    /// Returns an iterator over the contents of this deque in a forwards direction, assuming that
    /// we have already computed its forwards-facing list of elements via [`ensure_forwards`][].
    /// Panics if we haven't already computed it.
    ///
    /// [`ensure_forwards`]: #method.ensure_forwards
    pub fn iter_reused<'a>(&self, arena: &'a DequeArena<T>) -> impl Iterator<Item = &'a T> + 'a {
        let mut list = self.list;
        if self.is_backwards() {
            list.reverse_reused(arena)
                .expect("Forwards deque hasn't been calculated");
        }
        list.iter(arena)
    }

    /// Returns an iterator over the contents of this deque in a backwards direction, assuming that
    /// we have already computed its backwards-facing list of elements via [`ensure_backwards`][].
    /// Panics if we haven't already computed it.
    ///
    /// [`ensure_backwards`]: #method.ensure_backwards
    pub fn iter_reversed_reused<'a>(
        &self,
        arena: &'a DequeArena<T>,
    ) -> impl Iterator<Item = &'a T> + 'a {
        let mut list = self.list;
        if self.is_forwards() {
            list.reverse_reused(arena)
                .expect("Backwards deque hasn't been calculated");
        }
        list.iter(arena)
    }
}

// Normally we would #[derive] all of these traits, but the auto-derived implementations all
// require that T implement the trait as well.  We don't store any real instances of T inside of
// Deque, so our implementations do _not_ require that.

impl<T> Clone for Deque<T> {
    fn clone(&self) -> Deque<T> {
        Deque {
            list: self.list,
            direction: self.direction,
        }
    }
}

impl<T> Copy for Deque<T> {}
