//! Test module for common components

#[cfg(test)]
pub mod sparce_buffer_rc_test {
    use crate::client::common::handle::handle_t;
    use crate::client::common::sparce_buffer_rc::SparceBufferRc;

    #[test]
    fn test_allocation() {
        let mut buffer = SparceBufferRc::<i32>::new();
        let handle = buffer.Allocate(42);

        assert!(!handle.IsNull());
        assert_eq!(*buffer.GetMut(handle), 42);
    }
}
