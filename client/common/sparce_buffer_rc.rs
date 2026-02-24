use handle::handle_t;
use std::cell::{Cell, UnsafeCell};
use std::ops::{Deref, DerefMut};

#[repr(C)]
#[derive(Default)]

struct RcPtr<T: Default> {
    rc: i32,
    value: T,
}

struct Node<T: Default> {
    buffer: [RcPtr<T>; 64],
    bitmask: u64, //1 = free, 0 = taken
    next: i32,
}

impl<T: Default> Default for Node<T> {
    fn default() -> Self {
        return Self {
            buffer: std::array::from_fn(|_| RcPtr::default()),
            bitmask: 0xFFFFFFFFFFFFFFFF,
            next: -1,
        };
    }
}

pub struct SparceBufferRc<T: Default> {
    nodes: UnsafeCell<Vec<Box<Node<T>>>>,
    free_list: Cell<i32>,
}

pub struct BufferGuard<'a, T> {
    value: &'a mut T,
}

impl<'a, T> Deref for BufferGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, T> DerefMut for BufferGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<T: Default> SparceBufferRc<T> {
    pub fn new() -> Self {
        Self {
            nodes: UnsafeCell::new(Vec::new()),
            free_list: Cell::new(-1),
        }
    }

    pub fn Get(&self, h: handle_t<T>) -> BufferGuard<'_, T> {
        let node_idx = h.Node() as usize;
        let buf_idx = h.Value() as usize;

        unsafe {
            let vec_ptr = self.nodes.get();
            let node = &mut *(*vec_ptr)[node_idx];
            let value = &mut node.buffer[buf_idx].value;
            BufferGuard { value }
        }
    }

    pub fn Allocate(&self, default: T) -> handle_t<T> {
        unsafe {
            let nodes = &mut *self.nodes.get();

            if self.free_list.get() == -1 {
                let new_node = Box::new(Node::default());
                nodes.push(new_node);
                self.free_list.set((nodes.len() - 1) as i32);
            }

            let node_idx = self.free_list.get() as usize;
            let node = &mut *nodes[node_idx];

            let buffer_index = node.bitmask.trailing_zeros();
            let entry = &mut node.buffer[buffer_index as usize];
            entry.rc = 1;
            entry.value = default;

            node.bitmask &= !(1 << buffer_index);
            if node.bitmask == 0 {
                let next_free = node.next;
                self.free_list.set(next_free);
                node.next = -1;
            }

            handle_t::from(0, node_idx as u8, buffer_index as u8)
        }
    }

    pub fn BumpRef(&self, h: handle_t<T>) {
        unsafe {
            let nodes = &mut *self.nodes.get();
            if let Some(node) = nodes.get_mut(h.Node() as usize) {
                let rc_ptr = &mut node.buffer[h.Value() as usize];
                rc_ptr.rc += 1;
            }
        }
    }

    pub fn Free(&self, h: handle_t<T>) {
        let mut should_release = false;

        unsafe {
            let nodes = &mut *self.nodes.get();

            if let Some(node) = nodes.get_mut(h.Node() as usize) {
                let rc_ptr = &mut node.buffer[h.Value() as usize];

                rc_ptr.rc -= 1;

                if rc_ptr.rc == 0 {
                    should_release = true;
                }
            }
        }

        if should_release {
            self.Release(h);
        }
    }

    fn Release(&self, h: handle_t<T>) {
        unsafe {
            let nodes = &mut *self.nodes.get();
            let node_idx = h.Node() as usize;
            let slot_idx = h.Value() as usize;

            if let Some(node) = nodes.get_mut(node_idx) {
                node.bitmask |= 1 << slot_idx;
                let old_free_head = self.free_list.get();

                if old_free_head != node_idx as i32 {
                    node.next = old_free_head;
                    self.free_list.set(node_idx as i32);
                }
            }
        }
    }
}
