extern crate handle;
extern crate mesh;
extern crate transform;
use handle::handle_t;
use mesh::Mesh;
use transform::Transform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyType {
    RendererTransform,
    RendererMesh,
    RendererMaterial,
    RendererVisibility,
    PhysicsRigidBody,
    PhysicsCollider,
    PhysicsMass,
    NetworkPosition,
    NetworkVelocity,
    NetworkHealth,
    Null,
}

impl PropertyType {
    pub fn default_value(&self) -> PropertyValue {
        match self {
            PropertyType::RendererTransform => PropertyValue::RendererTransform(Transform::Identity()),
            PropertyType::RendererMesh => PropertyValue::RendererMesh(handle_t::null()),
            PropertyType::RendererMaterial => PropertyValue::RendererMaterial(Material {
                base_color: [1.0, 1.0, 1.0, 1.0],
                metallic: 0.0,
                roughness: 0.5,
            }),
            PropertyType::RendererVisibility => PropertyValue::RendererVisibility(true),
            PropertyType::PhysicsRigidBody => PropertyValue::PhysicsRigidBody(RigidBody {
                mass: 1.0,
                velocity: nalgebra::Vector3::zeros(),
                angular_velocity: nalgebra::Vector3::zeros(),
                is_kinematic: false,
            }),
            PropertyType::PhysicsCollider => PropertyValue::PhysicsCollider(Collider {
                shape: ColliderShape::Sphere(0.5),
                offset: nalgebra::Vector3::zeros(),
                is_trigger: false,
            }),
            PropertyType::PhysicsMass => PropertyValue::PhysicsMass(1.0),
            PropertyType::NetworkPosition => PropertyValue::NetworkPosition(nalgebra::Vector3::zeros()),
            PropertyType::NetworkVelocity => PropertyValue::NetworkVelocity(nalgebra::Vector3::zeros()),
            PropertyType::NetworkHealth => PropertyValue::NetworkHealth(100),
            PropertyType::Null => PropertyValue::Null,
        }
    }
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
        match self {
            PropertyValue::RendererTransform(_) => PropertyType::RendererTransform,
            PropertyValue::RendererMesh(_) => PropertyType::RendererMesh,
            PropertyValue::RendererMaterial(_) => PropertyType::RendererMaterial,
            PropertyValue::RendererVisibility(_) => PropertyType::RendererVisibility,
            PropertyValue::PhysicsRigidBody(_) => PropertyType::PhysicsRigidBody,
            PropertyValue::PhysicsCollider(_) => PropertyType::PhysicsCollider,
            PropertyValue::PhysicsMass(_) => PropertyType::PhysicsMass,
            PropertyValue::NetworkPosition(_) => PropertyType::NetworkPosition,
            PropertyValue::NetworkVelocity(_) => PropertyType::NetworkVelocity,
            PropertyValue::NetworkHealth(_) => PropertyType::NetworkHealth,
            PropertyValue::Null => PropertyType::Null,
        }
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
