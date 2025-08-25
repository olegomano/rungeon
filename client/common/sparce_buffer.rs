extern crate handle;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ops::{Index, IndexMut};

pub struct Node<T> {
    data: [MaybeUninit<T>; 32],
    free: i32,
}

const NULL_INDEX: u8 = 255;

struct SparceBuffer<T> {
    buffer: [UnsafeCell<Option<Box<Node<T>>>>; 255],

    next: [u8; 255],
    free_list: u8,
}

impl<T> SparceBuffer<T> {
    pub fn new() -> Self {
        return Self {
            buffer: std::array::from_fn(|_| UnsafeCell::new(None)),
            next: [0; 255],
            free_list: NULL_INDEX,
        };
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
