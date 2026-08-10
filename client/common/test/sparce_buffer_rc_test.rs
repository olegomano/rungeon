//! Comprehensive tests for SparceBufferRc

extern crate handle;
extern crate sparce_buffer_rc;

use sparce_buffer_rc::SparceBufferRc;
use std::cell::RefCell;

#[test]
fn test_basic() {
    // Create a buffer and test basic functionality
    let buffer = SparceBufferRc::<i32>::new();
    let handle = buffer.Allocate(42);

    assert!(!handle.IsNull());
    assert_eq!(*buffer.GetMut(handle), 42);
}

#[test]
fn test_direct_allocation() {
    let buffer = SparceBufferRc::<i32>::new();
    let handle = buffer.Allocate(42);

    assert!(!handle.IsNull());
    assert_eq!(*buffer.GetMut(handle), 42);
}

#[test]
fn test_minimal() {
    // This is a minimal test that should compile
    // We'll just verify that we can create the types

    // Create a buffer
    let _buffer = SparceBufferRc::<i32>::new();

    // If we got here without panicking, the test passes
    assert!(true);
}

#[test]
fn test_basic_allocation() {
    let buffer = SparceBufferRc::<i32>::new();
    let handle = buffer.Allocate(42);

    assert!(!handle.IsNull(), "Handle should not be null");
    assert_eq!(*buffer.GetMut(handle), 42, "Value should be 42");
}

#[test]
fn test_copy_functionality() {
    let buffer = SparceBufferRc::<i32>::new();
    let handle1 = buffer.Allocate(100);
    let handle2 = buffer.Copy(handle1);

    assert_eq!(*buffer.GetMut(handle1), 100, "Original value should be 100");
    assert_eq!(*buffer.GetMut(handle2), 100, "Copied value should be 100");
}

#[test]
fn test_free_functionality() {
    let buffer = SparceBufferRc::<i32>::new();
    let handle = buffer.Allocate(77);

    let destroyed = RefCell::new(false);
    let destructor = |v: i32| {
        assert_eq!(v, 77, "Destroyed value should be 77");
        *destroyed.borrow_mut() = true;
    };

    let result = buffer.Free(handle, destructor);
    assert!(result, "Free should return true when ref count reaches 0");
    assert!(*destroyed.borrow(), "Value should be destroyed");
}

#[test]
fn test_multiple_allocations() {
    let buffer = SparceBufferRc::<i32>::new();

    // Allocate multiple values
    let handle1 = buffer.Allocate(1);
    let handle2 = buffer.Allocate(2);
    let handle3 = buffer.Allocate(3);

    assert_eq!(*buffer.GetMut(handle1), 1, "First value should be 1");
    assert_eq!(*buffer.GetMut(handle2), 2, "Second value should be 2");
    assert_eq!(*buffer.GetMut(handle3), 3, "Third value should be 3");
}

#[test]
fn test_different_types() {
    // Test with different types
    let int_buffer = SparceBufferRc::<i32>::new();
    let int_handle = int_buffer.Allocate(42);
    assert_eq!(*int_buffer.GetMut(int_handle), 42);

    let float_buffer = SparceBufferRc::<f32>::new();
    let float_handle = float_buffer.Allocate(3.14);
    assert_eq!(*float_buffer.GetMut(float_handle), 3.14);
}

#[test]
fn test_reference_counting() {
    let buffer = SparceBufferRc::<i32>::new();
    let handle1 = buffer.Allocate(100);

    // After copy, ref count should be incremented
    let handle2 = buffer.Copy(handle1);

    // Free one reference, should not destroy
    let destroyed1 = RefCell::new(false);
    {
        let destructor = |_v: i32| {
            *destroyed1.borrow_mut() = true;
        };
        let result = buffer.Free(handle1, destructor);
        assert!(!result, "First free should not destroy");
        assert!(!*destroyed1.borrow(), "Value should not be destroyed yet");
    }

    // Free second reference, should destroy
    let destroyed2 = RefCell::new(false);
    {
        let destructor = |_v: i32| {
            *destroyed2.borrow_mut() = true;
        };
        let result = buffer.Free(handle2, destructor);
        assert!(result, "Second free should destroy");
        assert!(*destroyed2.borrow(), "Value should be destroyed");
    }
}
