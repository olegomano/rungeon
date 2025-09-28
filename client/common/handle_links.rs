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
    link_map: HashMap<handle_uuid, HashMap<TypeId, Vec<handle_uuid>>>,
}

impl LinkTable {
    pub fn new() -> Self {
        return Self {
            link_map: HashMap::new(),
        };
    }

    /**
     * There is hard assumption that there is only one link per type
     * IE we can't link two drawables to the same transform
     */
    pub fn Link<A: 'static, B: 'static>(
        &mut self,
        h1: handle::handle_t<A>,
        h2: handle::handle_t<B>,
    ) {
        let h1_uuid = handle_uuid::From(h1);
        let h2_uuid = handle_uuid::From(h2);

        self.link_map
            .entry(h1_uuid)
            .or_insert_with(|| HashMap::new())
            .entry(TypeId::of::<B>())
            .or_insert_with(|| Vec::new())
            .push(h2_uuid);
    }

    pub fn GetLinkedHandles<A: 'static, B: 'static>(
        &self,
        h1: handle::handle_t<A>,
    ) -> Vec<handle::handle_t<B>> {
        let h1_uuid = handle_uuid::From(h1);
        let type_id_b = std::any::TypeId::of::<B>();
        if let Some(type_map) = self.link_map.get(&h1_uuid) {
            if let Some(vec_uuids) = type_map.get(&type_id_b) {
                return vec_uuids
                    .iter()
                    .map(|uuid| handle::handle_t::<B>::new(uuid.handle_value))
                    .collect();
            }
        }
        return Vec::new();
    }
}
