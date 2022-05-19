use std::mem;
use std::os::raw::c_void;

/// Transmutes a raw mutable pointer into a mutable reference.
pub unsafe fn from_void_ptr<'ptr, F>(ptr: *mut c_void) -> &'ptr mut F {
    mem::transmute(ptr)
}

/// Places any value on the heap, producing a heap pointer to it.
/// Can leak memory if the pointer is never freed.
pub fn to_heap_ptr<F>(item: F) -> *mut c_void {
    Box::into_raw(Box::new(item)) as *mut c_void
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ptr_roundtripping() {
        let value: i32 = 1024;
        let expected: i32 = value.clone();

        let ptr: *mut c_void = to_heap_ptr(value);
        println!("Reconstituting value from the heap at {:?}.", ptr);
        let actual: &mut i32 = unsafe { from_void_ptr::<i32>(ptr) };
        // This is so the memory will get freed.
        let boxed: Box<i32> = unsafe { Box::from_raw(ptr as *mut i32) };

        assert_eq!(*actual, expected);
        assert_eq!(*boxed, expected);
        mem::forget(actual);
    }
}
