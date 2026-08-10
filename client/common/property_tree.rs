extern crate handle;
extern crate property;
extern crate sparce_buffer_rc;
use handle::handle_t;
use property::ObjectId;
use property::Property;
use property::PropertyId;
use property::PropertyKey;
use property::Version;
use std::collections::HashMap;

pub struct Object {
    object_id: ObjectId,
    members: HashMap<PropertyId, (handle::handle_t<Property>, Version)>,
}

/*
 *  Represents the meterialized state of a tree at a certain point in time
 */
pub struct TreeState {
    // object_id -> Object
    objects: HashMap<ObjectId, Object>,
}

#[derive(Clone)]
pub enum TreeDeltaPtr {
    Delta(handle_t<TreeDelta>),
    State(),
}

#[derive(Clone)]
pub struct TreeDelta {
    version: Version,
    mutations: HashMap<PropertyKey, handle::handle_t<Property>>,
    allocation: HashMap<PropertyKey, handle::handle_t<Property>>,
    deletion: HashMap<PropertyKey, handle::handle_t<Property>>,
    next: TreeDeltaPtr,
    prev: TreeDeltaPtr,
}

pub struct PropertyTree {
    property_buffer: HashMap<property::PropertyType, sparce_buffer_rc::SparceBufferRc<Property>>,
    delta_buffer: sparce_buffer_rc::SparceBufferRc<TreeDelta>,

    current_state: TreeState,
    prev_state: handle::handle_t<TreeDelta>,
    prev_state_finalized: bool,
    properties: HashMap<property::VersionedPropertyKey, handle::handle_t<Property>>,
    version: property::Version,
    object_id_generator : property::ObjectId,
    property_id_generator : property::PropertyId,
}

impl PropertyTree {
    pub fn new() -> Self {
        let mut tree = PropertyTree {
            property_buffer: HashMap::new(),
            delta_buffer: sparce_buffer_rc::SparceBufferRc::new(),
            current_state: TreeState {
                objects: HashMap::new(),
            },
            prev_state: handle::handle_t::null(),
            prev_state_finalized: false,
            properties: HashMap::new(),
            version: Version { id: 0 },
            object_id_generator : property::ObjectId{
                id : 0,
            },
            property_id_generator : property::PropertyId{
                id : 0,
            },
        };

        // Allocate the initial previous state delta
        let initial_delta = TreeDelta {
            version: Version { id: 0 },
            mutations: HashMap::new(),
            allocation: HashMap::new(),
            deletion: HashMap::new(),
            next: TreeDeltaPtr::State(),
            prev: TreeDeltaPtr::State(),
        };
        tree.prev_state = tree.delta_buffer.Allocate(initial_delta);
        tree
    }

    pub fn CreateObject(&mut self) -> property::ObjectId {
        // Generate a new unique ObjectId
        let object_id = property::ObjectId { id: self.object_id_generator.id };
        self.object_id_generator.id +=1;

        // Create a new Object and add it to the current state
        let object = Object {
            object_id,
            members: HashMap::new(),
        };

        self.current_state.objects.insert(object_id, object);
        object_id
    }

    pub fn DeleteObject(&mut self, object: property::ObjectId) {
        self.current_state.objects.remove(&object);
    }

    /*
     * Returns a PropertyKey
     */
    pub fn CreateProperty(
        &mut self,
        object: property::ObjectId,
        t: property::PropertyType,
        _v: property::PropertyValue,
    ) -> property::PropertyKey {
        let property_id = property::PropertyId { id: self.property_id_generator.id };
        self.property_id_generator.id +=1;

        property::PropertyKey {
            object_id: object,
            property_type: t,
            instance: property_id,
        }
    }

    /*
     * Returns a pointer to the current state of the tree
     * This just returns prev_state and then allocates a new TreeDelta and insets it as
     * the head. This sets prev_state_finalizde to be ture.
     */
    pub fn GetSnapshot(&mut self) -> handle::handle_t<TreeDelta> {
        handle::handle_t::null()
    }

    /*
     * Frees the snapshot so that the PropertyValue assosiated with the state can be
     * deleted
     */
    pub fn FreeSnapshot(&mut self, _s: handle::handle_t<TreeDelta>) {}

    /*
     * Writes the value in place into current_state, and then writes the value it replaced
     * into prev_state. If prev_state_finalized is true a new TreeDelta node is allocated.
     * Returns the PropertyKey that contains the specific version info
     */
    pub fn WriteProperty(&mut self, p: property::Property) -> property::Version {
        if let Some(object) = self.current_state.objects.get_mut(&p.key.object_id) {
            let property_version = property::Version {
                id: self.version.id + 1,
            };
            self.version.id += 1;

            let new_property_handle = self
                .property_buffer
                .entry(p.key.property_type.clone())
                .or_insert_with(|| sparce_buffer_rc::SparceBufferRc::new())
                .Allocate(p);

            self.properties.insert(
                property::VersionedPropertyKey {
                    key: p.key,
                    version: property_version,
                },
                new_property_handle,
            );

            //old_property_handle is the previous state we had for this property
            if let Some((old_property_handle, _old_version)) =
                object.members.insert(p.key.instance, (new_property_handle, property_version))
            {
                // Only track mutations if we have a valid previous state
                if !self.prev_state.IsNull() {
                    let delta = self.delta_buffer.GetMut(self.prev_state);
                    //if we already had a previous state tracked
                    // we can free it in this case since no-one has snapshotted it meaning no readers on it
                    if let Some(very_old_property_handle) =
                        delta.mutations.insert(p.key.clone(), old_property_handle)
                    {
                        self.property_buffer
                            .get_mut(&p.key.property_type)
                            .expect("Property buffer not found")
                            .Free(very_old_property_handle, |_v| {});

                        // Find and remove the old versioned property key
                        if let Some((key_to_remove, _)) = self
                            .properties
                            .iter()
                            .find(|(k, _)| k.key == p.key && k.version.id < property_version.id)
                        {
                            self.properties.remove(key_to_remove);
                        }
                    }
                } else {
                    // No previous state, just free the old property directly
                    self.property_buffer
                        .get_mut(&p.key.property_type)
                        .expect("Property buffer not found")
                        .Free(old_property_handle, |_v| {});
                }
            }
            return property_version;
        }
        property::Version::Null()
    }

    /*
     * Reads the property from the tree, If no version is supplied then will
     * the latest value
     */
    pub fn ReadProperty(&self, p: property::PropertyKey) -> property::PropertyValue {
        // Find the object
        if let Some(object) = self.current_state.objects.get(&p.object_id) {
            // Find the property in the object
            if let Some((handle, _)) = object.members.get(&p.instance) {
                // Get the property value
                if let Some(property_buffer) = self.property_buffer.get(&p.property_type) {
                    let property = property_buffer.GetMut(*handle);
                    return property.value.clone();
                }
            }
        }

        // Return a default value if not found
        property::PropertyValue::Null
    }

    /*
     * Given the state stapshot reconstruct the whole state at that time
     */
    pub fn MaterializeState(&mut self, _s: handle::handle_t<TreeDelta>) {
        // Walk the delta chain to reconstruct the state

        // Clear current state
        self.current_state.objects.clear();

        // Apply deltas in reverse order (from oldest to newest)
        // This is a simplified implementation
        // In a real implementation, you'd need to walk the linked list
        // and apply changes in the correct order
    }

    /*
     *  Starts from the lower version handle and walks the TreeDelta linked list
     *  Till it reaches the other handle.
     */
    pub fn Delta<F>(
        &self,
        _p1: handle::handle_t<TreeDelta>,
        _p2: handle::handle_t<TreeDelta>,
        _callback: F,
    ) where
        F: Fn(&property::Property, &property::Property),
    {
        // Walk from p1 to p2 and collect all changes

        // This is a simplified implementation
        // In a real implementation, you would:
        // 1. Walk the linked list from p1 to p2
        // 2. Collect all property changes
        // 3. Call the callback for each changed property
    }

    /*
     * Triggers the callback for each property in the given object
     * Runs over the most current state
     */
    pub fn ForEachPropertyInObject<F>(&self, object_id: property::ObjectId, f: F)
    where
        F: Fn(&property::Property),
    {
        if let Some(object) = self.current_state.objects.get(&object_id) {
            for (property_id, (handle, _)) in &object.members {
                // We need to find which property buffer contains this handle
                // by iterating through all buffers
                for property_buffer in self.property_buffer.values() {
                    let property = property_buffer.GetMut(*handle);
                    if property.key.instance == *property_id {
                        f(property);
                        break;
                    }
                }
            }
        }
    }
}
