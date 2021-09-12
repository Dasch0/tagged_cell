# tagged\_cell
[![CI](https://github.com/Dasch0/tagged_cell/actions/workflows/rust.yml/badge.svg)](https://github.com/Dasch0/tagged_cell/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/tagged_cell.svg)](https://crates.io/crates/tagged_cell)
[![API reference](https://docs.rs/once_cell/badge.svg)](https://docs.rs/tagged_cell/)

Fast, initializable, and thread safe static variables


## Overview
Borrows the excellent ZST based tagging implementation (linked below) to guarantee the cell
is initialized exactly once before an access is attempted.
<https://www.hardmo.de/article/2021-03-14-zst-proof-types.md#proof-of-work>

This is implemented via `TaggedCell` and a Tag type, which **must** be unique for
each instance of the `TaggedCell` for safe operation. The `TaggedCell` must then be initialized
via `TaggedCell::init()`, which initializes the underlying data using a user provided function or closure,
and then returns a special zero-sized `Init<Tag>` used to access the Cell's data.

To ensure unique tag types are used for each cell, the tagged\_cell! macro is provided. The
macro creates a new tag type based on the variable's name, and applies it in the declaration.
```rust
use tagged_cell::tagged_cell;
tagged_cell!{
   static BAR: TaggedCell<Vec<usize>, _> = TaggedCell::new();
}

let tag = BAR.init(|| vec![0, 10, 20]);
let vec = BAR.get(tag);

assert_eq!(vec[2], 20);
```

To allow for usage across threads, only the first invocation of TaggedCell::init will initialize the
Cell's data. All future TaggedCell::init calls will just return a new tag. It is undetermined which
thread will initialize the Cell's data.
```rust
use std::thread;
use tagged_cell::tagged_cell;

tagged_cell!{
    static TABLE: TaggedCell<Vec<usize>, _> = TaggedCell::new();
}

thread::spawn(move || {
    let tag = TABLE.init(|| vec![0, 10, 20]);
    let table = TABLE.get(tag);
    assert_eq!(table[2], 20);
});

thread::spawn(move || {
    let tag = TABLE.init(|| vec![0, 10, 20]);
    let table = TABLE.get(tag);
    assert_eq!(table[1], 10);
});

```

