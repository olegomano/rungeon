extern crate handle;
use std::any::TypeId;
use std::collections::HashMap;

/*
 * handles are scoped within their type
 *
 * because of this to link across types we must make a uuid for the handle
 * that also inclues its type info
 *
 */
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct handle_uuid {
    handle_value: i16,
    type_id: TypeId,
}

impl handle_uuid {
    fn From<T: 'static>(handle: handle::handle_t<T>) -> Self {
        return Self {
            handle_value: handle.Value(),
            type_id: TypeId::of::<T>(),
        };
    }
}

/*
 * Table that provides mapping between associated types
 *
 * IE if we have a Drawable, it should have a pose, but those are seperate
 * allocations. So we keep a link map that associates them that we can do
 * queries against
 */
pub struct LinkTable {
    link_map: HashMap<handle_uuid, HashMap<i32, handle_uuid>>,
}

impl LinkTable {
    pub fn new() -> Self {
        return Self {
            link_map: HashMap::new(),
        };
    }

    pub fn Link<A, B>(&mut self, h1: handle::handle_t<A>, h2: handle::handle_t<B>) {
        let h1_uuid = handle_uuid::From(h1);
        let h2_uuid = handle_uuid::From(h2);
    }

    pub fn GetLinkedHandle<A, B>(&self, h1: handle::handle_t<A>) -> handle::handle_t<B> {}
}
