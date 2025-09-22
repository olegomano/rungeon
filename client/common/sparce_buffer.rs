extern crate handle;
use std::cell::Cell;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ops::{Index, IndexMut};

const NULL_INDEX: u8 = 255;
const NODE_SIZE: usize = 32;
const NODE_COUNT: usize = 255;

pub struct Node<T> {
    data: [MaybeUninit<T>; NODE_SIZE],
    free: u32,
}

pub struct SparceBuffer<T> {
    buffer: [UnsafeCell<Option<Box<Node<T>>>>; NODE_COUNT],

    next: [Cell<u8>; NODE_COUNT],
    free_list: Cell<u8>,
    alloc_count: Cell<u8>,
}

pub struct SparceBufferIter<'a, T> {
    buffer: &'a SparceBuffer<T>,
    index: i32,
    iter_count: i32,
}

impl<T> SparceBuffer<T> {
    pub fn new() -> Self {
        let mut next: [Cell<u8>; 255] = core::array::from_fn(|i| Cell::new(i as u8));
        for i in 0..255 {
            next[i].set((i + 1) as u8);
        }
        next[254].set(NULL_INDEX);

        return Self {
            buffer: std::array::from_fn(|_| UnsafeCell::new(None)),
            next: next,
            free_list: Cell::new(0),
            alloc_count: Cell::new(0),
        };
    }

    pub fn Iter(&self) -> SparceBufferIter<T> {
        return SparceBufferIter {
            buffer: self,
            index: 0,
            iter_count: 0,
        };
    }

    pub fn Allocate(&self, value: T) -> handle::handle_t<T> {
        if self.free_list.get() == NULL_INDEX {
            return handle::handle_t::null();
        }
        let buffer = self.GetNode(self.free_list.get());

        let leading_zero = buffer.free.leading_zeros();
        let index = 31 - leading_zero;
        buffer[index as usize] = value;
        buffer.free = (buffer.free & !(1 << index));
        self.alloc_count.set(self.alloc_count.get() + 1);

        let result = handle::handle_t::from(1, self.free_list.get() as u8, index as u8);

        if buffer.free == 0 {
            //we are full
            self.free_list
                .set(self.next[self.free_list.get() as usize].get());
        }
        return result;
    }

    pub fn Size(&self) -> usize {
        return self.alloc_count.get() as usize;
    }

    pub fn Free(&self, h: handle::handle_t<T>) {
        if (h.IsNull()) {
            return;
        }
        self.free_list.set(h.Node());
        self.next[h.Node() as usize].set(self.free_list.get());
        let mut node = self.GetNode(h.Node());
        node.free = node.free | (1 << h.Instance());

        self.alloc_count.set(self.alloc_count.get() - 1);
    }

    fn GetNode(&self, node: u8) -> &mut Node<T> {
        let mut optional_box = unsafe { &mut *self.buffer[node as usize].get() };
        if optional_box.is_none() {
            // 1 is free 0 is used
            *optional_box = Some(Box::new(Node::<T> {
                data: unsafe { MaybeUninit::uninit().assume_init() },
                free: 0xffffffff as u32,
            }));
        }
        return optional_box.as_deref_mut().expect("");
    }
}

impl<'a, T> Iterator for SparceBufferIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while true {
            //we already iterated all the nodes
            if self.iter_count >= self.buffer.Size() as i32 {
                return None;
            }

            let node = self.index as usize / NODE_SIZE;
            let instance = self.index as usize % NODE_SIZE;
            //made it to the end
            if node >= NODE_COUNT {
                return None;
            }
            self.index = self.index + 1;

            let instance_mask = 1 << instance;
            if (self.buffer.GetNode(node as u8).free & instance_mask) == 0 {
                self.iter_count = self.iter_count + 1;
                return Some(&self.buffer.GetNode(node as u8)[instance]);
            }
        }
        return None;
    }
}

impl<T> Index<usize> for Node<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        unsafe {
            return &*self.data[index].as_ptr();
        }
    }
}

impl<T> IndexMut<usize> for Node<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        unsafe {
            return &mut *self.data[index].as_mut_ptr();
        }
    }
}

impl<T> Index<handle::handle_t<T>> for SparceBuffer<T> {
    type Output = T;

    fn index(&self, index: handle::handle_t<T>) -> &T {
        unsafe {
            let optional_box = &*self.buffer[index.Node() as usize].get();
            let node = optional_box.as_deref().unwrap();
            return &node[index.Instance() as usize];
        }
    }
}

impl<T> IndexMut<handle::handle_t<T>> for SparceBuffer<T> {
    fn index_mut(&mut self, index: handle::handle_t<T>) -> &mut T {
        unsafe {
            let optional_box = &mut *self.buffer[index.Node() as usize].get();
            let node: &mut Node<T> = optional_box.as_deref_mut().unwrap();
            return &mut node[index.Instance() as usize];
        }
    }
}
