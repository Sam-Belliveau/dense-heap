# DHeap (Dense Heap) Allocator and DBox (Dense Box) Smart Pointer

This project provides a custom memory allocator called `DHeap` and a smart pointer called `DBox`. The primary goal of this allocator is to minimize memory fragmentation by densely packing the allocated memory.

## Features

- Minimizes memory fragmentation
- Minimizes memory usage for uniformly sized allocations
- Smart pointer `DBox` for easy memory management

## Usage

To use this custom allocator, first, create a `DHeap` instance with the desired capacity:

```rust
let heap: DHeap<i32> = DHeap::with_capacity(16);
```

To allocate memory in the `DHeap`, you can use the `safe_new` method:

```rust
let dbox = heap.safe_new(42).unwrap();
```

The `DBox` smart pointer is used to access and manage the data stored in the `DHeap`. You can dereference the `DBox` to access the underlying data:

```rust
assert_eq!(*dbox, 42);
```

The `DBox` smart pointer automatically deallocates the memory when it goes out of scope or when the `into_inner` method is called:

```rust
let inner_val = dbox.into_inner();
```

## Example

A basic example of using the DHeap allocator and DBox smart pointer can be found in the tests module within the source code.

## Safety

The code uses unsafe Rust features to optimize performance, but these are limited and accompanied by explanations. The use of `DBox` ensures that the memory management is safe and prevents issues like double frees or use-after-free. However, be cautious when using the `unsafe_new` method, as it may invalidate existing references if the underlying vector needs to be resized.
