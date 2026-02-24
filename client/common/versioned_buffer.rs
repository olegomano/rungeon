use std::collections::HashMap;
use std::collections::HashSet;
extern crate handle;
extern crate sparce_buffer_rc;

#[derive(Debug)]
enum TrieValue<T: Copy + std::default::Default> {
    Empty,
    Trie(handle::handle_t<TrieNode<T>>),
    Leaf {
        key: i64,
        value: handle::handle_t<T>,
    },
}

impl<T: Copy + std::default::Default> Copy for TrieValue<T> {}

impl<T: Copy + std::default::Default> Clone for TrieValue<T> {
    fn clone(&self) -> TrieValue<T> {
        return *self;
    }
}

impl<T> Default for TrieValue<T>
where
    T: Default + Copy,
{
    fn default() -> Self {
        return TrieValue::Empty;
    }
}

#[derive(Debug)]
struct TrieNode<T: Copy + std::default::Default> {
    children: [TrieValue<T>; 256],
    node_hash: i64,
    level: i64,
}

impl<T> Default for TrieNode<T>
where
    T: Default + Copy,
{
    fn default() -> Self {
        return Self {
            children: std::array::from_fn(|_| TrieValue::default()),
            node_hash: 0,
            level: 0,
        };
    }
}

impl<T> TrieNode<T>
where
    T: Default + Copy,
{
    //returns the index in the children array this key maps to
    fn KeyIndex(&self, key: i64) -> i64 {
        let mask: i64 = 0xFF << self.level * 8;
        return mask & key;
    }
}

impl<T: Copy + std::default::Default> Copy for TrieNode<T> {}

impl<T: Copy + std::default::Default> Clone for TrieNode<T> {
    fn clone(&self) -> TrieNode<T> {
        return *self;
    }
}

pub struct Trie<T: std::default::Default + Copy> {
    node_allocator: sparce_buffer_rc::SparceBufferRc<TrieNode<T>>,
    data_allocator: sparce_buffer_rc::SparceBufferRc<T>,
    root_node: handle::handle_t<TrieNode<T>>,
}

pub struct Commit<T> {
    pending_writes: Vec<(i64, T)>,
}

impl<T> Trie<T>
where
    T: Default + Copy,
{
    pub fn new() -> Self {
        let mut result = Self {
            root_node: handle::handle_t::null(),
            node_allocator: sparce_buffer_rc::SparceBufferRc::new(),
            data_allocator: sparce_buffer_rc::SparceBufferRc::new(),
        };

        result.root_node = result.node_allocator.Allocate(TrieNode::<T>::default());
        return result;
    }

    pub fn ApplyCommit(&mut self, commit: Commit<T>) -> handle::handle_t<TrieNode<T>> {
        let new_root = self.CopyNode(self.root_node);
        let mut inserted_nodes = HashSet::new();
        let mut removed_nodes = HashSet::new();

        inserted_nodes.insert(new_root);
        removed_nodes.insert(self.root_node);

        for (key, value) in commit.pending_writes.iter() {
            self.CopyPathImpl(*key, new_root, &mut inserted_nodes, &mut removed_nodes);
        }
        for (key, value) in commit.pending_writes.iter() {
            self.WriteValueImpl(*key, *value, new_root);
        }

        for node : removed_nodes.iter() {
            self.node_allocator.Free(node);
        }

        return new_root;
    }

    pub fn Snapshot(&self) -> handle::handle_t<TrieNode<T>> {
        return self.root_node;
    }

    pub fn ReleaseSnapshot(&self, h: handle::handle_t<TrieNode<T>>) {}

    /*
     * Find the difference between the state of two trie nodes
     * For every key that is different in each root, we invoke a callback
     *
     * left is old, right is new
     * if a null handle is passed for either arg then we invoke the callback for each element
     */
    pub fn Diff<F>(
        &self,
        a: handle::handle_t<TrieNode<T>>,
        b: handle::handle_t<TrieNode<T>>,
        mut cb: F,
    ) where
        F: FnMut(i64, handle::handle_t<T>, handle::handle_t<T>),
    {
    }

    /*
     * Returns a new root that contains the same data but a new set of nodes on the path to key
     */
    fn CopyPath(
        &self,
        key: i64,
        inserted_nodes: &mut HashSet<handle::handle_t<TrieNode<T>>>,
        removed_nodes: &mut HashSet<handle::handle_t<TrieNode<T>>>,
    ) -> handle::handle_t<TrieNode<T>> {
        let new_root = self.CopyNode(self.root_node);
        inserted_nodes.insert(new_root);
        removed_nodes.insert(self.root_node);
        self.CopyPathImpl(key, new_root, inserted_nodes, removed_nodes);
        return new_root;
    }

    //We assume the parent coming into this function is the copied parent so it can be mutated in place
    //To update the pointer for the key
    fn CopyPathImpl(
        &self,
        key: i64,
        parent: handle::handle_t<TrieNode<T>>,
        copied_nodes: &mut HashSet<handle::handle_t<TrieNode<T>>>,
        removed_nodes: &mut HashSet<handle::handle_t<TrieNode<T>>>,
    ) {
        let index = self.GetKeyIndex(key, parent);
        match self.GetKeyValue(key, parent) {
            TrieValue::Empty => {} //no key found, should not happen
            TrieValue::Trie(h) => {
                if (!copied_nodes.contains(&h)) {
                    let self_copy = self.CopyNode(h);
                    copied_nodes.insert(self_copy);
                    removed_nodes.insert(h);
                    self.WriteKeyValue(key, parent, TrieValue::Trie(self_copy));
                }
                let self_copy = copied_nodes.get(&h).expect("");
                self.CopyPathImpl(key, *self_copy, copied_nodes, removed_nodes);
            }
            TrieValue::Leaf {
                key: leaf_key,
                value,
            } => {} //found the key, stop iteration
        }
    }

    //Writes the value in place
    //You should have copied the tree with CopyPathImpl first and the wrote into the copy
    //This will traverse down to the Leaf and write the value in place
    //If the leaf has another key written in it, it will make a new Node and move both keys into it
    fn WriteValueImpl(&self, key: i64, value: T, parent: handle::handle_t<TrieNode<T>>) {
        let index = self.node_allocator.Get(parent).KeyIndex(key) as usize;
        match self.node_allocator.Get(parent).children[index] {
            TrieValue::Empty => {
                let new_value = self.data_allocator.Allocate(value);
                self.WriteKeyValue(
                    key,
                    parent,
                    TrieValue::<T>::Leaf {
                        key: key,
                        value: new_value,
                    },
                );
            }
            TrieValue::Trie(h) => {
                self.WriteValueImpl(key, value, h);
            }
            TrieValue::Leaf {
                key: leaf_key,
                value: leaf_value,
            } => {
                //found they key, it is either us and we over-write it
                //or its a differnet value and we need to make a new node and split it
                if (key == leaf_key) {
                    let new_value = self.data_allocator.Allocate(value);
                    let leaf = TrieValue::<T>::Leaf {
                        key: key,
                        value: new_value,
                    };
                    self.WriteKeyValue(key, parent, leaf);
                } else {
                    //when making the new node both keys may land in the same index again,
                    //so we must keep recursively calling this
                    let new_child = self.node_allocator.Allocate(TrieNode::<T>::default());
                    self.node_allocator.Get(new_child).level =
                        self.node_allocator.Get(parent).level + 1;

                    self.WriteKeyValue(key, parent, TrieValue::Trie(new_child));
                    self.WriteKeyValue(
                        key,
                        new_child,
                        TrieValue::<T>::Leaf {
                            key: leaf_key,
                            value: leaf_value,
                        },
                    );
                    self.WriteValueImpl(key, value, new_child);
                }
            }
        }
    }

    /*
     * Given a node, allocate a new Node and copy all the pointers/values into it
     */
    fn CopyNode(&self, h: handle::handle_t<TrieNode<T>>) -> handle::handle_t<TrieNode<T>> {
        let new_node = self.node_allocator.Allocate(*self.node_allocator.Get(h));
        return new_node;
    }

    fn WriteKeyValue(&self, key: i64, parent: handle::handle_t<TrieNode<T>>, value: TrieValue<T>) {
        let mut node = self.node_allocator.Get(parent);
        let index = node.KeyIndex(key) as usize;
        node.children[index] = value;
    }

    fn GetKeyValue(&self, key: i64, parent: handle::handle_t<TrieNode<T>>) -> TrieValue<T> {
        let mut node = self.node_allocator.Get(parent);
        let index = node.KeyIndex(key) as usize;
        return node.children[index];
    }

    fn GetKeyIndex(&self, key: i64, parent: handle::handle_t<TrieNode<T>>) -> i64 {
        let node = self.node_allocator.Get(parent);
        return node.KeyIndex(key);
    }
}
