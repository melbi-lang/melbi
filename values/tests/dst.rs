use std::{
    alloc::Layout,
    marker::PhantomData,
    ptr::{self, NonNull},
    slice,
};

use bumpalo::Bump;

#[repr(C)]
struct FlexSlice<H, S: ?Sized> {
    header: H,
    slice: S,
}

#[repr(C)]
struct FlexSliceWithLength<H, T> {
    len: usize,
    data: FlexSlice<H, [T; 0]>, // Must be sized for a thin pointer.
}

#[repr(C)]
pub struct Flex<'a, H, T> {
    inner: NonNull<FlexSliceWithLength<H, T>>,
    phantom: PhantomData<&'a ()>,
}

impl<'a, H, T> Flex<'a, H, T> {
    fn from_iter(
        arena: &'a bumpalo::Bump,
        header: H,
        iter: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
    ) -> Self {
        let mut iter = iter.into_iter();
        let n = iter.len();
        let (layout, slice_offset) = Self::layout(n);
        let storage = arena.alloc_layout(layout);
        unsafe {
            let ptr = storage.as_ptr().cast::<FlexSliceWithLength<H, T>>();
            ptr::write(
                ptr,
                FlexSliceWithLength {
                    len: n,
                    data: FlexSlice { header, slice: [] },
                },
            );
            let slice_ptr: *mut T = storage.add(slice_offset).as_ptr().cast();
            for i in 0..n {
                ptr::write(
                    slice_ptr.add(i),
                    iter.next().expect("iterator exhausted too early"),
                );
            }
        }
        Self {
            inner: storage.cast(),
            phantom: PhantomData,
        }
    }

    fn as_fat_ref(&self) -> &FlexSlice<H, [T]> {
        unsafe {
            let inner: &FlexSliceWithLength<H, T> = self.inner.as_ref();
            let len = inner.len;
            let p = &inner.data as *const _ as *const T;
            let dst = slice::from_raw_parts(p, len);
            let ret = dst as *const _ as *const FlexSlice<H, [T]>;
            &*ret
        }
    }

    fn layout(n: usize) -> (Layout, usize) {
        let (layout, slice_offset) = Layout::new::<FlexSliceWithLength<H, T>>()
            .extend(Layout::array::<T>(n).unwrap())
            .unwrap();
        (layout.pad_to_align(), slice_offset)
    }
}

#[test]
fn test_flex_inner() {
    let arena = Bump::new();
    let a = Flex::from_iter(&arena, "something", [1, 2, 3]);
    assert_eq!(a.as_fat_ref().header, "something");
    assert_eq!(&a.as_fat_ref().slice, &[1, 2, 3]);
}
