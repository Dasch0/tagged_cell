#![doc = include_str!("../README.md")]
use std::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, sync::Once};

/// Top level structure to support initializable and thread safe static variables.
/// Use [tagged_cell!] macro to make this struct
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
    /// Internal method to create an uninitialized cell. This is an implementation detail and
    /// *should not* be called directly and thus is listed as unsafe. Use [tagged_cell!] instead
    /// This must only be called once for each `Tag` type.
    #[doc(hidden)]
    pub const unsafe fn new() -> Self {
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

        static $name: $crate::TaggedCell<$type, $name::TagType> = unsafe {$crate::TaggedCell::new()};
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
