// dheap.rs --- dense heap implementation.

// Copyright (c) 2023 Sam Belliveau. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::{
    cell::{Cell, UnsafeCell},
    hint::unreachable_unchecked,
    marker::PhantomData,
    mem::{replace, ManuallyDrop},
    ops::{Deref, DerefMut, Drop},
};

/// The DHeapNode contains all the metadata required to keep the DHeap organized.
enum DHeapNode<T: Sized> {
    /// Edge is always the last element of the vector. When the
    /// head points to the edge, new memory must be allocated.
    Edge(),

    /// Empty represents a previously occupied slot that has
    /// been freed. It points to the previous head when
    /// it was freed, creating a chain of free blocks for
    /// future allocations.
    Empty(usize),

    /// Holding represents a memory slot in use by a DBox<_>.
    /// The memory is owned by the DBox<_> pointing to it,
    /// which is why it is wrapped in a ManuallyDrop<_>. The DBox<_>
    /// is guaranteed to drop before the DHeap<_>.
    Holding(ManuallyDrop<T>),

    /// When calling DBox.into_inner(), memory is moved out of the
    /// DHeap<_> before the DBox<_> has dropped. This serves as an indicator
    /// for the DBox<_> not to panic when it finds its memory moved during
    /// the dropping process.
    Moved(),
}

use DHeapNode::*;

/// A DHeap is a dense heap data structure that efficiently manages memory allocation and deallocation.
///
/// The heap has an overhead of 24 bytes per element, and it will never use more memory than what is allocated
/// at any given point in time, no matter which elements are freed and in which order. The linking nature of the
/// indices will always backfill optimally, ensuring that the memory usage is as efficient as possible.
pub struct DHeap<T: Sized> {
    buffer: UnsafeCell<Vec<DHeapNode<T>>>,
    head: Cell<usize>,
}

impl<T> DHeap<T> {
    /// Creates a new `DHeap` with a specified initial capacity.
    ///
    /// Allocates a buffer with the requested capacity, plus one additional element to account for the `Edge`.
    /// The `Edge` is a sentinel element used to facilitate certain heap operations.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The desired initial capacity for the heap.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is less than or equal to 1, as the heap requires at least 2 elements to function properly.
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 1);

        DHeap {
            buffer: {
                // We add one more element than requested to account for the Edge.
                let mut memory = Vec::with_capacity(capacity + 1);
                memory.push(Edge());
                memory.into()
            },
            head: Cell::new(0),
        }
    }

    // internally used to make life easy
    fn memory(&self) -> &mut Vec<DHeapNode<T>> {
        unsafe { &mut *self.buffer.get() }
    }

    /// Allocates memory for the given value `v` in the `DHeap` and returns a `DBox` pointing to it.
    ///
    /// This function is marked `unsafe` because it may potentially invalidate existing references
    /// if the underlying vector needs to be resized. However, `DBox` instances will still function correctly.
    ///
    /// When the end of the free block list is reached, a new element is pushed during allocation. If this
    /// new element requires the vector to grow, any existing references to elements within the dense heap
    /// might become invalid. This risk should be carefully considered when using this heap.
    ///
    /// One approach to mitigate this risk is to use safe_new().
    ///
    /// # Safety
    ///
    /// Users must ensure that no references to elements within the dense heap are held when calling this function.
    /// If references are held, they may become invalid after the function call.
    pub unsafe fn unsafe_new(&self, v: T) -> DBox<T> {
        let index = self.head.get();

        match self.memory()[index] {
            Edge() => {
                // The implementation's weak point lies in this push operation, which is unavoidable.
                // When the end of the free block list is reached, a new element must be pushed
                // during allocation. If the new element causes the vector to grow, it leads to a problem:
                // any references to elements within the dense heap become invalid.
                // It's crucial to carefully consider this risk when using this heap.
                self.head.set(self.size());
                self.memory().push(Edge());
            }

            Empty(next) => self.head.set(next),
            _ => panic!("invalid head pointer! [corrupted memory]"),
        }

        self.memory()[index] = Holding(ManuallyDrop::new(v));

        DBox {
            heap: self,
            index,
            _marker: PhantomData,
        }
    }

    /// Provides a safe alternative to `DHeap::new()` by attempting to allocate
    /// memory without resizing the underlying vector.
    ///
    /// This function ensures that no existing references will be invalidated during
    /// the allocation process, as it only allocates memory when there is available
    /// capacity within the reserved memory. However, if the reserved memory is
    /// exhausted, an error is returned.
    ///
    /// # Returns
    ///
    /// - `Ok(DBox<T>)` if the allocation was successful.
    /// - `Err(&'static str)` if there is no available capacity within the reserved memory.
    pub fn safe_new(&self, v: T) -> Result<DBox<T>, &'static str> {
        if self.memory().len() == self.memory().capacity() {
            Err("out of reserved memory!")
        } else {
            // SAFETY: The vector is not resized, so no existing references are invalidated.
            unsafe { Ok(self.unsafe_new(v)) }
        }
    }

    /// Retrieves the current memory usage of the `DHeap`.
    ///
    /// This function returns the number of elements in the underlying vector,
    /// which represents the total memory occupied by the `DHeap`.
    ///
    /// # Returns
    ///
    /// - A `usize` representing the memory usage of the `DHeap`.
    pub fn size(&self) -> usize {
        self.memory().len()
    }
}

/// DBox is a smart pointer designed to work with the DHeap allocator.
///
/// It provides similar functionality to Box in the Rust standard library but is specifically tailored
/// for use with the dense heap implementation (DHeap).
pub struct DBox<'a, T> {
    heap: &'a DHeap<T>,
    index: usize,
    _marker: PhantomData<T>,
}

impl<'a, T> DBox<'a, T> {
    fn data(&self) -> &'a DHeapNode<T> {
        &self.heap.memory()[self.index]
    }

    fn mut_data(&mut self) -> &'a mut DHeapNode<T> {
        &mut self.heap.memory()[self.index]
    }

    /// Consumes the `DBox` and retrieves the inner value `T`.
    ///
    /// This function replaces the `DBox`'s memory cell with a `Moved` state, indicating
    /// that the memory has been moved out of the `DHeap` before the `DBox` is dropped.
    /// After replacing the cell, it returns the inner value of the `DBox`.
    ///
    /// # Returns
    ///
    /// - The inner value `T` contained within the `DBox`.
    pub fn into_inner(mut self) -> T {
        match replace(self.mut_data(), Moved()) {
            Holding(value) => ManuallyDrop::into_inner(value),
            _ => panic!("use after free! [corrupted memory]"),
        }
    }
}

impl<'a, T> Drop for DBox<'a, T> {
    fn drop(&mut self) {
        match self.mut_data() {
            Holding(value) => {
                // SAFETY: The memory cell is immediately replaced with an empty cell after dropping.
                unsafe { ManuallyDrop::drop(value) }
            }
            Moved() => {}
            _ => panic!("double free! [corrupted memory]"),
        }

        *self.mut_data() = Empty(self.heap.head.replace(self.index));
    }
}

impl<'a, T> Deref for DBox<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        if let Holding(value) = self.data() {
            value.deref()
        } else {
            // SAFETY:
            // This code is frequently executed, so we use unsafe code to bypass the match.
            // This should never be reached unless memory corruption occurs, but the
            // compiler isn't aware of this guarantee.
            unsafe { unreachable_unchecked() }
        }
    }
}

impl<'a, T> DerefMut for DBox<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Holding(value) = self.mut_data() {
            value.deref_mut()
        } else {
            // SAFETY:
            // This code is frequently executed, so we use unsafe code to bypass the match.
            // This should never be reached unless memory corruption occurs, but the
            // compiler isn't aware of this guarantee.
            unsafe { unreachable_unchecked() }
        }
    }
}

impl<'a, T> AsRef<T> for DBox<'a, T> {
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<'a, T> AsMut<T> for DBox<'a, T> {
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}
