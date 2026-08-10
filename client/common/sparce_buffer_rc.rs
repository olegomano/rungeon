use std::alloc::Layout;
use std::cell::Cell;
use std::cell::UnsafeCell;
use std::ptr::NonNull;

struct ValueRefCount<T: Clone> {
    ref_count: i32,
    value: T,
}

struct MemChunk<T: Clone> {
    ptr: NonNull<ValueRefCount<T>>,
    bitfield: u32,
}

impl<T: Clone> MemChunk<T> {
    fn capacity() -> usize {
        32
    }

    fn new() -> Self {
        let new_layout = Layout::array::<ValueRefCount<T>>(Self::capacity()).unwrap();
        let ptr = unsafe {
            let raw_ptr = std::alloc::alloc(new_layout) as *mut ValueRefCount<T>;
            NonNull::new(raw_ptr).expect("Allocation failed")
        };
        Self {
            ptr: ptr,
            bitfield: 0xFFFFFFFF, // All 32 bits set for 32 slots
        }
    }

    fn GetMut<'a>(&'a mut self, index: usize) -> &'a mut T {
        debug_assert!(index < Self::capacity(), "Index out of bounds");
        unsafe {
            let ptr = self.ptr.as_ptr().add(index);
            &mut (*ptr).value
        }
    }

    fn Free(&self) -> bool {
        self.bitfield != 0
    }

    fn Allocate(&mut self, v: T) -> Option<u8> {
        if !self.Free() {
            return None;
        }
        // Find the highest set bit (31 - leading_zeros gives us 0-31)
        let highest_bit = self.bitfield.leading_zeros();
        let index = 31 - highest_bit;
        unsafe {
            let dest_ptr = self.ptr.as_ptr().add(index as usize);
            std::ptr::write(
                dest_ptr,
                ValueRefCount {
                    ref_count: 1,
                    value: v,
                },
            );
        }
        self.bitfield &= !(1u32 << index);
        Some(index as u8)
    }

    fn IncrementRc(&mut self, index: usize) -> i32 {
        debug_assert!(index < Self::capacity(), "Index out of bounds");
        unsafe {
            let ptr = self.ptr.as_ptr().add(index);
            (*ptr).ref_count += 1;
            (*ptr).ref_count
        }
    }

    fn DecrementRc(&mut self, index: usize) -> i32 {
        debug_assert!(index < Self::capacity(), "Index out of bounds");
        unsafe {
            let ptr = self.ptr.as_ptr().add(index);
            (*ptr).ref_count -= 1;
            (*ptr).ref_count
        }
    }

    fn FreeChunk(&mut self) {
        let element_count = Self::capacity();
        let layout = Layout::array::<ValueRefCount<T>>(element_count).unwrap();
        unsafe {
            std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
        }
    }
}

/*
 *  Represents a slab based allocator that is reference counted
 */
pub struct SparceBufferRc<T: Clone> {
    chunk_buffer: UnsafeCell<Vec<MemChunk<T>>>,
    free_chunks: Cell<i32>,
}

impl<T: Clone> Drop for SparceBufferRc<T> {
    fn drop(&mut self) {
        let chunks = unsafe { &mut *self.chunk_buffer.get() };
        for mut chunk in chunks.drain(..) {
            chunk.FreeChunk();
        }
    }
}

impl<T: Clone> SparceBufferRc<T> {
    pub fn new() -> Self {
        Self {
            chunk_buffer: UnsafeCell::new(Vec::new()),
            free_chunks: Cell::new(0),
        }
    }

    pub fn Allocate(&self, value: T) -> handle::handle_t<T> {
        // Get the chunks vector
        let chunk_buffer = unsafe { &mut *self.chunk_buffer.get() };

        // Find a chunk with free space
        let mut chunk_index = None;
        for (index, chunk) in chunk_buffer.iter_mut().enumerate() {
            if chunk.Free() {
                chunk_index = Some(index);
                break;
            }
        }

        // If no chunk with free space found, create a new one
        let chunk_index = if let Some(index) = chunk_index {
            index
        } else {
            let new_chunk = MemChunk::<T>::new();
            chunk_buffer.push(new_chunk);
            chunk_buffer.len() - 1
        };

        // Decrement free chunks count
        let current_free = self.free_chunks.get();
        self.free_chunks.set(current_free - 1);

        // Allocate in the selected chunk
        let chunk = &mut chunk_buffer[chunk_index];
        let instance = chunk.Allocate(value).expect("Allocation failed");
        handle::handle_t::from(0, chunk_index as u8, instance)
    }

    pub fn Copy(&self, h: handle::handle_t<T>) -> handle::handle_t<T> {
        let chunks = self.MutChunks();
        debug_assert!(
            (h.Node() as usize) < chunks.len(),
            "Node index out of bounds"
        );
        chunks[h.Node() as usize].IncrementRc(h.Instance() as usize);
        h
    }

    pub fn GetMut<'a>(&'a self, h: handle::handle_t<T>) -> &'a mut T {
        let chunks = self.MutChunks();
        debug_assert!(
            (h.Node() as usize) < chunks.len(),
            "Node index out of bounds"
        );
        chunks[h.Node() as usize].GetMut(h.Instance() as usize)
    }

    pub fn Free<F>(&self, h: handle::handle_t<T>, destructor: F) -> bool
    where
        F: FnOnce(T),
    {
        let chunks = self.MutChunks();
        debug_assert!(
            (h.Node() as usize) < chunks.len(),
            "Node index out of bounds"
        );
        let rc = chunks[h.Node() as usize].DecrementRc(h.Instance() as usize);
        if rc == 0 {
            // Get the value before freeing
            let value_ptr = unsafe {
                chunks[h.Node() as usize]
                    .ptr
                    .as_ptr()
                    .add(h.Instance() as usize)
            };
            let value = unsafe { std::ptr::read(value_ptr) };
            destructor(value.value);
            // Mark the slot as free in the bitfield
            chunks[h.Node() as usize].bitfield |= 1u32 << h.Instance();
            // Increment free chunks count
            let current_free = self.free_chunks.get();
            self.free_chunks.set(current_free + 1);
            true
        } else {
            false
        }
    }

    fn MutChunks(&self) -> &mut Vec<MemChunk<T>> {
        unsafe { &mut *self.chunk_buffer.get() }
    }
}
