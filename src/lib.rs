//! Initializable & thread safe static variables, with zero performance overhead for reads
//!
//! Borrows the excellent ZST based tagging implementation (linked below) to guarantee the cell
//! is initialized exactly once before an access is attempted.
//! <https://www.hardmo.de/article/2021-03-14-zst-proof-types.md#proof-of-work>
//!
//! This is implemented via the [TaggedCell] and a Tag type, which **must** be unique for
//! each instance of the [TaggedCell] for safe operation. The [TaggedCell] must then be initialized
//! via [init()][TaggedCell::init], which initializes the underlying data using a user provided function or closure,
//! and then returns a special zero-sized [Init] tag used to access the Cell's data.
//! ```
//! use tagged_cell::TaggedCell;
//! struct FooTag;
//! static FOO: TaggedCell<usize, FooTag> = TaggedCell::new();
//!
//! // Initialize the cell's data and retrieve a tag
//! let tag = FOO.init(|| 27);
//!
//! // The tag is required to obtain a shared reference to the data
//! assert_eq!(*FOO.get(tag), 27);
//!
//! ```
//!
//! To ensure unique tag types are used for each cell, the [tagged_cell!] macro is recommended. The
//! macro creates a new tag type based on the variable's name, and applies it in the declaration.
//! ```
//! use tagged_cell::tagged_cell;
//! tagged_cell!{
//!    static BAR: TaggedCell<Vec<usize>, _> = TaggedCell::new();
//! }
//!
//! let tag = BAR.init(|| vec![0, 10, 20]);
//! let vec = BAR.get(tag);
//!
//! assert_eq!(vec[2], 20);
//! ```
//!
//! When unique tag types are used, attempting to access a [TaggedCell] before it is initialized
//! will cause a compilation error.
//! ```compile_fail
//! use tagged_cell::tagged_cell;
//!
//! tagged_cell!{
//!     static BAZ: TaggedCell<usize>, _> = TaggedCell::new();
//! }
//! tagged_cell!{
//!     static QUX: TaggedCell<usize>, _> = TaggedCell::new();
//! }
//!
//! // read before init is not possible
//! BAZ.get(Init{BAZ::TagType});
//!
//! let qux_tag = QUX.init(|| 35);
//!
//! // using the wrong tag throws an error
//! BAZ.get(qux_tag);
//! ```
//!
//! To allow for usage across threads, only the first invocation of [init()][TaggedCell::init] will initialize the
//! Cell's data. All future [init()][TaggedCell::init] calls will just return a new tag. It is undetermined which
//! thread will initialize the Cell's data.
//! ```
//! use std::thread;
//! use tagged_cell::tagged_cell;
//!
//! tagged_cell!{
//!     static TABLE: TaggedCell<Vec<usize>, _> = TaggedCell::new();
//! }
//!
//! thread::spawn(move || {
//!     let tag = TABLE.init(|| vec![0, 10, 20]);
//!     let table = TABLE.get(tag);
//!     assert_eq!(table[2], 20);
//! });
//!
//! thread::spawn(move || {
//!     let tag = TABLE.init(|| vec![0, 10, 20]);
//!     let table = TABLE.get(tag);
//!     assert_eq!(table[1], 10);
//! });
//!
//! ```
//!
use std::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, sync::Once};

/// Top level structure to support initializable and thread safe static variables.
/// It is recommended to use [tagged_cell!] macro to make this struct
pub struct TaggedCell<T, Tag> {
    once: Once,
    tag: PhantomData<Tag>,
    data: UnsafeCell<MaybeUninit<T>>,
}

/// A marker proving that the unique cell with tag `Tag` is initialized.
/// This cannot be sent across threads, the only way to obtain it is by running
/// [init()][TaggedCell::init] in the current thread
#[derive(Clone, Copy)]
pub struct Init<Tag> {
    tag: PhantomData<Tag>,
}

impl<T, Tag> TaggedCell<T, Tag> {
    /// Make an uninitialized cell.
    /// This must only be called once for each `Tag` type.
    pub const fn new() -> Self {
        TaggedCell {
            data: UnsafeCell::new(MaybeUninit::<T>::uninit()),
            tag: PhantomData,
            once: Once::new(),
        }
    }

    /// Initialize a TaggedOnceCell. This function initializes the cell, if not already
    /// initialized, using the provided function or closure. Additionally returns a zero-sized Tag,
    /// which is required to access the underlying data.
    ///
    /// Each thread accessing a TaggedOnceCell should call this method to obtain a Tag, the
    /// initialization code will only run once. It is undetermined which thread will run the
    /// initialization code.
    pub fn init<F>(&self, f: F) -> Init<Tag>
    where
        F: Fn() -> T,
    {
        unsafe {
            self.once.call_once(|| {
                let mut_data = &mut *self.data.get();
                mut_data.write(f());
            });
        }
        Init { tag: self.tag }
    }

    /// Get the data within a [TaggedCell], requires an initialized tag to perform the access
    #[inline(never)]
    pub fn get(&self, _: Init<Tag>) -> &T {
        // SAFETY: Init tag proves that `init` has successfully
        // returned before in the current thread, initializing the cell.
        unsafe {
            let maybe_val = &mut *self.data.get();
            maybe_val.assume_init_ref()
        }
    }
}

/// [TaggedCell] may be Sync. Guaranteed by ZST tag
unsafe impl<T: Sync + Send, Tag> Sync for TaggedCell<T, Tag> {}
unsafe impl<T: Send, Tag> Send for TaggedCell<T, Tag> {}

/// Macro for creating a [TaggedCell]
#[macro_export]
macro_rules! tagged_cell {
    (static $name:ident : TaggedCell<$type:ty, _> = TaggedCell::new();) => {
        #[allow(non_snake_case)]
        mod $name {
            #[allow(dead_code)]
            pub struct TagType;
        }

        static $name: $crate::TaggedCell<$type, $name::TagType> = $crate::TaggedCell::new();
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn simple() {
        tagged_cell! {
            static TEST: TaggedCell<usize, _> = TaggedCell::new();
        }

        let tag = TEST.init(|| 0);
        let num = TEST.get(tag);

        assert_eq!(*num, 0);
    }
}
