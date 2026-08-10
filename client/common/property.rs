#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RendererProperty {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicsProperty {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetworkProperty {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyType {
    Renderer(RendererProperty),
    Physics(PhysicsProperty),
    Network(NetworkProperty),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    pub id: u64,
}

impl Version {
    pub fn Null() -> Self {
        return Version { id: 0 };
    }

    pub fn IsNull(&self) -> bool {
        return self.id == 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId {
    pub id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyId {
    pub id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyKey {
    pub instance: PropertyId,
    pub property_type: PropertyType,
    pub object_id: ObjectId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VersionedPropertyKey {
    pub key: PropertyKey,
    pub version: Version,
}

#[derive(Debug, Clone)]
pub enum PropertyValue {
    String(String),
    Boolean(bool),
    Integer(i32),
    Float(f32),
    Property(PropertyKey),
    PropertyArray(Vec<PropertyKey>),
    Null,
}

#[derive(Debug, Clone)]
pub struct Property {
    pub key: PropertyKey,
    pub value: PropertyValue,
    pub version: Version,
}
