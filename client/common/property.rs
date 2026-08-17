extern crate handle;
extern crate mesh;
extern crate transform;
use handle::handle_t;
use mesh::Mesh;
use transform::Transform;
use std::mem::{discriminant, Discriminant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyType(pub Discriminant<PropertyValue>);

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

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    RendererTransform(Transform),
    RendererMesh(handle_t<Mesh>),
    RendererMaterial(Material),
    RendererVisibility(bool),
    PhysicsRigidBody(RigidBody),
    PhysicsCollider(Collider),
    PhysicsMass(f32),
    NetworkPosition(nalgebra::Vector3<f32>),
    NetworkVelocity(nalgebra::Vector3<f32>),
    NetworkHealth(u32),
    Null,
}

impl PropertyValue {
    pub fn Type(&self) -> PropertyType {
        PropertyType(discriminant(self))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RigidBody {
    pub mass: f32,
    pub velocity: nalgebra::Vector3<f32>,
    pub angular_velocity: nalgebra::Vector3<f32>,
    pub is_kinematic: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Collider {
    pub shape: ColliderShape,
    pub offset: nalgebra::Vector3<f32>,
    pub is_trigger: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColliderShape {
    Box(nalgebra::Vector3<f32>),
    Sphere(f32),
    Capsule { radius: f32, height: f32 },
}

#[derive(Debug, Clone)]
pub struct Property {
    pub key: PropertyKey,
    pub value: PropertyValue,
    pub version: Version,
}
