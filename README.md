# tagged\_cell
[![CI](https://github.com/Dasch0/tagged_cell/actions/workflows/rust.yml/badge.svg)](https://github.com/Dasch0/tagged_cell/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/tagged_cell.svg)](https://crates.io/crates/tagged_cell)
[![API reference](https://docs.rs/once_cell/badge.svg)](https://docs.rs/tagged_cell/)

Fast, initializable, and thread safe static variables

Borrows the [excellent ZST based tagging implementation](https://www.hardmo.de/article/2021-03-14-zst-proof-types.md#proof-of-work)
 to guarantee cells
are initialized exactly once before an access is attempted.

This is implemented via the [TaggedCell] and a Tag type, which **must** be unique for
each instance of the [TaggedCell] for safe operation. The [TaggedCell] must then be set up 
via [init()][TaggedCell::init], which initializes the underlying data using a user provided function or closure,
and then returns a special zero-sized [Init] tag used to access the Cell's data.

```
use tagged_cell::TaggedCell;
struct FooTag;
static FOO: TaggedCell<usize, FooTag> = unsafe {TaggedCell::new()};

// Initialize the cell's data and retrieve a tag
let tag = FOO.init(|| 27);

// The tag is required to obtain a shared reference to the data
assert_eq!(*FOO.get(tag), 27);

```

To ensure unique tag types are used for each cell, and to 'wrap' the unsafe call, the [tagged_cell!] macro is provided. The
macro creates a new tag type based on the variable's name, and applies it in the declaration.
```
use tagged_cell::tagged_cell;
tagged_cell!{
   static BAR: TaggedCell<Vec<usize>, _> = TaggedCell::new();
}

let tag = BAR.init(|| vec![0, 10, 20]);
let vec = BAR.get(tag);

assert_eq!(vec[2], 20);
```

When unique tag types are used, attempting to access a [TaggedCell] before it is initialized
will cause a compilation error.
```compile_fail
use tagged_cell::tagged_cell;

tagged_cell!{
    static BAZ: TaggedCell<usize>, _> = TaggedCell::new();
}
tagged_cell!{
    static QUX: TaggedCell<usize>, _> = TaggedCell::new();
}

// read before init is not possible
BAZ.get(Init{BAZ::TagType});

let qux_tag = QUX.init(|| 35);

// using the wrong tag throws an error
BAZ.get(qux_tag);
```

To allow for usage across threads, only the first invocation of [init()][TaggedCell::init] will initialize the
Cell's data. All future [init()][TaggedCell::init] calls will just return a new tag. It is undetermined which
thread will initialize the Cell's data.
```
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

