# Folder Context
This folder contains common utilits and core components that are re-used
across all module implementations and server as a common interface of data types that modules can use to communicate with eachother 

Everything is expected to be built with Bazel. If you want to see how to add
third party libraries as dependencies, consult MODULE.bazel

# Code Style
These are core components that are meant to be performant and re-usable libraries. They should leverage zero copy, performance and data oriented techinques as much as possible

Every library should contain test that go in the test subdirectory. Each libraries tests
should go into one file with the name of LIBRARY_NAME_test.rs

# Libraries
## handle.rs
  The core pointer type. It internally contains a handle that the owning
  container can interpret any way it wants. Although they are typed we support having an untyped void* style versoin that is up to the caller to make sure is casted correctly

## sparce_buffer.rs
  The core container for storing things. It returns a handle_t to the allocated data. Internally it keeps a linked list of blocks as the backing for the allocation. It provides efficient alloc/free and allows iteration of allocated objects
  
  ### API
  * Allocate() -> handle_t
  * Free(handle_t)
  * Iterator

## transform.rs
  Represents a matrix4x4 and provides common interactons such as translation, rotation and look at

## property.rs
  The api primitive that represnets a member field of a game state object. It is an enum that can one of {boolean, string, handle_t, float, int, transform, peropertyKey}. 
  
  It is a managed field that is tracked through a PropertyKey. There is a central repository tracking all Properties allocated in the game state called the PropertyTree. 

  ### PropertyKey
  - instance: the global uuid of this property
  - property_type: the type of property it is
  - owner: represents the object that owns this property
  
  ### PropertyValue
    Represents all the possible vaules a property can be
    - string
    - bool
    - i32 
    - f32
    - PropertyKey
    - Vec<PropertyKey>
  
  ### PropertyType
    Represents the type+member of this property
    Each enum value represents a submodule or "class" and then each embedded
    enum represents the member within that class
  
## property_store.rs
  Stores a colleciton of properties. 
  Responcible for generating the uuid for PropertyKeys

  ### API
    - NewProperty(owner: i32,value: PropertyValue) -> Property
    
## property_tree.rs
  The actual storage of the PropertyKey and PropertyValue. The game state will be stored as a collection of Properties.The idea is that it can track change of propery values over time.

  Every time a frame ticks each submodule will observe to property tree and only action on properties that have changed from the last frame. 

  ### API
    - WriteProperty(p : Property)
    - ReadProperty(k : PropertyKey, v : Version) -> PropertyValue
    - GetSnapshot() -> Snapshot()
      - Generates a snapshot of the current state
    - Delta(v_a :  Snapshot, v_b : Snahpshot, cb : f(p1,p2))
      - Given two snahshots trigger a callback for all properties that changed between the two 
    - ForPropertyInObject(owner : i32, cb : f(Property))
      - Given a owner id, iterate each property in it
    - ForEachProperty(type : PropertyType, f(Property))
      - Given a proprety type iterate all instances of this property
