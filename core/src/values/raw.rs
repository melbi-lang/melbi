#![allow(
    unsafe_code,
    reason = "low-level arena memory layout and raw pointer representations for dynamic values"
)]

use core::fmt;
use core::ptr::NonNull;

use bumpalo::Bump;

use crate::values::Function;

#[repr(C)]
pub union RawValue {
    // TODO: make all fields private.
    int_value: i64,
    float_value: f64,
    bool_value: bool,
    ptr: *const (),
    pub boxed: *const RawValue,
    array: *const ArrayDataRepr,
    record: *const RecordDataRepr,
    map: *const MapDataRepr,
    pub slice: *const Slice,
    option: Option<NonNull<RawValue>>,
    function: *const (), // Thin pointer to arena-allocated fat pointer
    function_new: NonNull<DynTraitHeader<dyn for<'a> Function<'a, 'a>>>,
}

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
static_assertions::assert_eq_size!(RawValue, usize);

impl Copy for RawValue {}
impl Clone for RawValue {
    fn clone(&self) -> Self {
        *self
    }
}

impl RawValue {
    /// Create an Option value at the raw level.
    ///
    /// This encapsulates the memory layout of Option values, ensuring a single
    /// source of truth. If the representation changes, only this function needs updating.
    //
    // TODO: This is not as efficient as it could be. Ideally, we want to box unboxed values,
    // but values already boxed do not need to be boxed again.
    #[inline]
    pub fn make_optional(arena: &Bump, value: Option<Self>) -> Self {
        RawValue {
            option: value.map(|v| NonNull::from(arena.alloc(v))),
        }
    }

    #[inline(always)]
    #[must_use]
    pub fn make_bool(value: bool) -> Self {
        RawValue { bool_value: value }
    }

    #[inline(always)]
    #[must_use]
    pub fn make_int(value: i64) -> Self {
        RawValue { int_value: value }
    }

    #[inline(always)]
    #[must_use]
    pub fn make_float(value: f64) -> Self {
        RawValue { float_value: value }
    }

    #[inline(always)]
    #[must_use]
    pub fn as_optional_unchecked(&self) -> Option<Self> {
        unsafe { self.option.map(|p| *p.as_ref()) }
    }

    #[inline(always)]
    #[must_use]
    pub fn as_int_unchecked(self) -> i64 {
        unsafe { self.int_value }
    }

    #[inline(always)]
    #[must_use]
    pub fn as_float_unchecked(self) -> f64 {
        unsafe { self.float_value }
    }

    #[inline(always)]
    #[must_use]
    pub fn as_bool_unchecked(self) -> bool {
        unsafe { self.bool_value }
    }

    #[inline(always)]
    #[must_use]
    pub fn as_bytes_unchecked<'a>(self) -> &'a [u8] {
        unsafe { (*self.slice).as_slice() }
    }

    #[inline(always)]
    #[must_use]
    pub fn as_str_unchecked<'a>(self) -> &'a str {
        unsafe { core::str::from_utf8_unchecked(self.as_bytes_unchecked()) }
    }

    /// Create a function value using a single allocation containing both the fat pointer
    /// and the function object.
    ///
    /// The memory layout is:
    ///
    /// ```text
    /// [*const dyn Function (16 bytes)][T object (sizeof<T> bytes)]
    ///  ^                               ^
    ///  |                               |
    ///  thin pointer stored             fat pointer's data points here
    ///  in RawValue.function
    /// ```
    ///
    /// # Arguments
    /// * `arena` - Arena to allocate the combined storage
    /// * `func` - The function value to store (will be moved into the allocation)
    ///
    /// # Returns
    /// A `RawValue` representing the allocated function.
    pub fn make_function<'a, 'b, F: Function<'a, 'b> + 'b>(arena: &'b Bump, func: F) -> Self {
        let (layout, value_offset) = {
            let ptr_layout = core::alloc::Layout::new::<*const dyn Function<'a, 'b>>();
            let value_layout = core::alloc::Layout::new::<F>();
            let (layout, value_offset) = ptr_layout.extend(value_layout).unwrap();
            (layout.pad_to_align(), value_offset)
        };
        let storage = arena.alloc_layout(layout);

        // Initialize the allocation:
        // 1. Write the function object T at offset `value_offset`
        // 2. Write the fat pointer at the beginning, pointing to the T object
        unsafe {
            let func_ptr = storage.add(value_offset).as_ptr().cast::<F>();
            core::ptr::write(func_ptr, func);

            // Create fat pointer: Rust constructs vtable when casting T* to dyn Function*
            let fat_ptr: *const dyn Function<'a, 'b> = func_ptr;
            core::ptr::write(
                storage.as_ptr().cast::<*const dyn Function<'a, 'b>>(),
                fat_ptr,
            );
        };

        RawValue {
            function: storage.as_ptr() as *const (),
        }
    }

    // TODO: Implement this and replace the old implementation.
    // pub fn make_function_new<'b, F: Function<'b, 'b> + 'b>(arena: &'b Bump, func: F) -> RawValue {
    //     let header = DynTraitNode::<dyn Function, F>::new(arena, func);
    //     RawValue {
    //         function_new: header,
    //     }
    // }

    /// Extract a function trait object reference from this `RawValue`.
    ///
    /// # Safety
    ///
    /// The caller must ensure this `RawValue` was created with `make_function`
    /// and contains a valid function pointer.
    #[inline(always)]
    #[must_use]
    pub fn as_function_unchecked<'a, 'b>(self) -> &'a dyn Function<'b, 'a> {
        let storage_ptr = unsafe { self.function.cast::<*const dyn Function<'b, 'a>>() };
        unsafe { &**storage_ptr }
    }

    /// Returns an id associated with this `RawValue`.
    ///
    /// For boxed values, if `id(a) == id(b)` then `a == b`.
    #[inline(always)]
    #[must_use]
    pub fn id(&self) -> usize {
        unsafe { self.ptr as usize }
    }
}

impl fmt::Debug for RawValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:p}", unsafe { self.boxed })
    }
}

#[repr(C)]
pub struct ArrayDataRepr {
    length: usize,
    data: [RawValue; 0],
}

#[derive(Clone, Copy)]
pub struct ArrayData<'a> {
    ptr: *const ArrayDataRepr,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> ArrayData<'a> {
    fn new_uninitialized_in(arena: &'a Bump, length: usize) -> (*mut ArrayDataRepr, *mut RawValue) {
        let (layout, data_offset) = Self::layout(length);

        unsafe {
            let ptr = arena.alloc_layout(layout).as_ptr();
            core::ptr::write::<usize>(ptr.cast::<usize>(), length);
            let data = ptr.add(data_offset).cast::<RawValue>();
            let array_data_ptr = ptr.cast::<ArrayDataRepr>();
            (array_data_ptr, data)
        }
    }

    pub fn new_with(arena: &'a Bump, values: &[RawValue]) -> Self {
        let (arr, data_ptr) = Self::new_uninitialized_in(arena, values.len());
        for (i, &val) in values.iter().enumerate() {
            unsafe { core::ptr::write(data_ptr.add(i), val) };
        }
        ArrayData {
            ptr: arr,
            _marker: core::marker::PhantomData,
        }
    }

    fn layout(n: usize) -> (core::alloc::Layout, usize) {
        let array_data_layout = core::alloc::Layout::new::<usize>();
        let elements_layout = core::alloc::Layout::array::<RawValue>(n).unwrap();
        let (layout, data_offset) = array_data_layout.extend(elements_layout).unwrap();
        (layout.pad_to_align(), data_offset)
    }

    #[must_use]
    pub fn length(&self) -> usize {
        unsafe { (*self.ptr).length }
        // unsafe { *(self.ptr as *const ArrayDataRepr as *const usize) }
    }

    /// Returns a pointer to the first element of the `data` array.
    #[must_use]
    pub fn as_data_ptr(&self) -> *const RawValue {
        let (_, data_offset) = Self::layout(self.length());
        unsafe { self.ptr.cast::<u8>().add(data_offset).cast::<RawValue>() }
    }

    /// # Safety
    ///
    /// The caller must ensure that `index` is less than `self.length()`.
    #[must_use]
    pub unsafe fn get_unchecked(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length(), "Index out of bounds");
        unsafe { *self.as_data_ptr().add(index) }
    }

    pub(crate) fn as_raw_value(&self) -> RawValue {
        RawValue { array: self.ptr }
    }

    pub(crate) fn from_raw_value(raw: RawValue) -> Self {
        ArrayData {
            ptr: unsafe { raw.array },
            _marker: core::marker::PhantomData,
        }
    }
}

#[repr(C)]
pub struct RecordDataRepr {
    length: usize,
    data: [RawValue; 0],
}

#[derive(Clone, Copy)]
pub struct RecordData<'a> {
    ptr: *const RecordDataRepr,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> RecordData<'a> {
    fn new_uninitialized_in(
        arena: &'a Bump,
        length: usize,
    ) -> (*mut RecordDataRepr, *mut RawValue) {
        let (layout, data_offset) = Self::layout(length);

        unsafe {
            let ptr = arena.alloc_layout(layout).as_ptr();
            core::ptr::write::<usize>(ptr.cast::<usize>(), length);
            let data = ptr.add(data_offset).cast::<RawValue>();
            let record_data_ptr = ptr.cast::<RecordDataRepr>();
            (record_data_ptr, data)
        }
    }

    pub fn new_with(arena: &'a Bump, values: &[RawValue]) -> Self {
        let (rec, data_ptr) = Self::new_uninitialized_in(arena, values.len());
        for (i, &val) in values.iter().enumerate() {
            unsafe { core::ptr::write(data_ptr.add(i), val) };
        }
        RecordData {
            ptr: rec,
            _marker: core::marker::PhantomData,
        }
    }

    fn layout(n: usize) -> (core::alloc::Layout, usize) {
        let record_data_layout = core::alloc::Layout::new::<usize>();
        let elements_layout = core::alloc::Layout::array::<RawValue>(n).unwrap();
        let (layout, data_offset) = record_data_layout.extend(elements_layout).unwrap();
        (layout.pad_to_align(), data_offset)
    }

    #[must_use]
    pub fn length(&self) -> usize {
        unsafe { (*self.ptr).length }
    }

    pub(self) fn as_ptr(&self) -> *const RawValue {
        let (_, data_offset) = Self::layout(self.length());
        unsafe { self.ptr.cast::<u8>().add(data_offset).cast::<RawValue>() }
    }

    /// # Safety
    ///
    /// The caller must ensure that `index` is less than `self.length()`.
    #[must_use]
    pub unsafe fn get(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length(), "Index out of bounds");
        unsafe { *self.as_ptr().add(index) }
    }

    pub(crate) fn as_raw_value(&self) -> RawValue {
        RawValue { record: self.ptr }
    }

    pub(crate) fn from_raw_value(raw: RawValue) -> Self {
        RecordData {
            ptr: unsafe { raw.record },
            _marker: core::marker::PhantomData,
        }
    }
}

#[repr(C)]
pub struct Slice {
    data: *const u8,
    length: usize,
}

impl Slice {
    pub fn new<'a>(arena: &'a Bump, value: &[u8]) -> &'a Self {
        arena.alloc(Self {
            data: value.as_ptr(),
            length: value.len(),
        })
    }

    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.data, self.length) }
    }

    pub(crate) fn as_raw_value(&self) -> RawValue {
        RawValue {
            slice: core::ptr::from_ref::<Self>(self),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MapEntry {
    pub key: RawValue,
    pub value: RawValue,
}

#[repr(C)]
pub struct MapDataRepr {
    length: usize, // Number of key-value pairs
    data: [MapEntry; 0],
}

#[derive(Clone, Copy)]
pub struct MapData<'a> {
    ptr: *const MapDataRepr,
    _marker: core::marker::PhantomData<&'a ()>,
}

impl<'a> MapData<'a> {
    fn new_uninitialized_in(arena: &'a Bump, length: usize) -> (*mut MapDataRepr, *mut MapEntry) {
        let (layout, data_offset) = Self::layout(length);

        unsafe {
            let ptr = arena.alloc_layout(layout).as_ptr();
            core::ptr::write::<usize>(ptr.cast::<usize>(), length);
            let data = ptr.add(data_offset).cast::<MapEntry>();
            let map_data_ptr = ptr.cast::<MapDataRepr>();
            (map_data_ptr, data)
        }
    }

    /// Create a new map from sorted key-value pairs.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - Keys are sorted in ascending order according to `Value::cmp`
    pub fn new_with_sorted(arena: &'a Bump, entries: &[MapEntry]) -> Self {
        let length = entries.len();
        let (map, data_ptr) = Self::new_uninitialized_in(arena, length);

        for (i, &entry) in entries.iter().enumerate() {
            unsafe { core::ptr::write(data_ptr.add(i), entry) };
        }

        MapData {
            ptr: map,
            _marker: core::marker::PhantomData,
        }
    }

    fn layout(n: usize) -> (core::alloc::Layout, usize) {
        let map_data_layout = core::alloc::Layout::new::<usize>();
        let elements_layout = core::alloc::Layout::array::<MapEntry>(n).unwrap();
        let (layout, data_offset) = map_data_layout.extend(elements_layout).unwrap();
        (layout.pad_to_align(), data_offset)
    }

    /// Returns the number of key-value pairs in the map.
    #[must_use]
    pub fn length(&self) -> usize {
        unsafe { (*self.ptr).length }
    }

    pub(crate) fn as_ptr(&self) -> *const MapEntry {
        let (_, data_offset) = Self::layout(self.length());
        unsafe { self.ptr.cast::<u8>().add(data_offset).cast::<MapEntry>() }
    }

    /// Get the key at the given index.
    ///
    /// # Safety
    ///
    /// The caller must ensure index < `length()`.
    #[must_use]
    pub unsafe fn get_key(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length(), "Index out of bounds");
        unsafe { (*self.as_ptr().add(index)).key }
    }

    /// Get the value at the given index.
    ///
    /// # Safety
    ///
    /// The caller must ensure index < `length()`.
    #[must_use]
    pub unsafe fn get_value(&self, index: usize) -> RawValue {
        debug_assert!(index < self.length(), "Index out of bounds");
        unsafe { (*self.as_ptr().add(index)).value }
    }

    pub(crate) fn as_raw_value(&self) -> RawValue {
        RawValue { map: self.ptr }
    }

    pub(crate) fn from_raw_value(raw: RawValue) -> Self {
        MapData {
            ptr: unsafe { raw.map },
            _marker: core::marker::PhantomData,
        }
    }
}

// This is how we access the dyn trait while type-erasing the concrete type.
// We use repr(C) to ensure it matches the prefix of the Node.
#[repr(C)]
struct DynTraitHeader<T: ?Sized> {
    dyn_ptr: NonNull<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: the following abstractions/ideas should be used in the production
    // code above, since they standardize the method used.
    // Must be implemented by any trait that we want to use with DynTraitNode.
    trait AsDyn<T: ?Sized> {
        fn as_dyn(&mut self) -> &mut T;
    }

    impl<'a, S: Function<'a, 'a> + 'a> AsDyn<dyn Function<'a, 'a> + 'a> for S {
        fn as_dyn(&mut self) -> &mut (dyn Function<'a, 'a> + 'a) {
            self
        }
    }

    // This is what actually gets allocated.
    #[repr(C)]
    struct DynTraitNode<T: ?Sized, U: AsDyn<T>> {
        dyn_ptr: Option<NonNull<T>>,
        obj: U, // The concrete object
    }

    // Ensure layout compatibility between Option<NonNull<T>> and NonNull<T>
    // for the cast in DynTraitNode::new to be sound.
    static_assertions::assert_eq_size!(
        Option<NonNull<dyn Function<'_, '_>>>,
        NonNull<dyn Function<'_, '_>>
    );

    impl<T: ?Sized, U: AsDyn<T>> DynTraitNode<T, U> {
        #[expect(
            clippy::new_ret_no_self,
            reason = "Constructor returns NonNull pointer to header"
        )]
        pub fn new(arena: &Bump, obj: U) -> NonNull<DynTraitHeader<T>> {
            // Two-phase init: allocate first, then create fat pointer from stable address
            let node: &mut Self = arena.alloc(Self { dyn_ptr: None, obj });
            let fat_ref: &mut T = node.obj.as_dyn(); // Use AsDyn<T> trait.
            node.dyn_ptr = Some(NonNull::from(fat_ref));
            NonNull::from(node).cast()
        }
    }

    trait MyTrait {
        fn foo(&self) -> i32;
    }
    // Blanket implementation for all types that implement MyTrait.
    impl<'a, S: MyTrait + 'a> AsDyn<dyn MyTrait + 'a> for S {
        fn as_dyn(&mut self) -> &mut (dyn MyTrait + 'a) {
            self
        }
    }

    struct MyStruct<'a>(&'a str);

    impl MyTrait for MyStruct<'_> {
        fn foo(&self) -> i32 {
            self.0.len() as i32
        }
    }

    #[test]
    fn dyn_trait_node_works() {
        let arena = Bump::new();
        let header_ptr =
            DynTraitNode::<dyn MyTrait, MyStruct>::new(&arena, MyStruct(arena.alloc_str("hello")));
        let header: &DynTraitHeader<dyn MyTrait> = unsafe { header_ptr.as_ref() };
        let result = unsafe { header.dyn_ptr.as_ref() }.foo();

        assert_eq!(result, 5);
    }
}
