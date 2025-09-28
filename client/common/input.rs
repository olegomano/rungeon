extern crate nalgebra;
use nalgebra::{Matrix4, Point3, Quaternion, Unit, UnitQuaternion, Vector3, Vector4};

#[derive(Debug, Clone, PartialEq)]
pub struct MotionInput {
    pub movement: Vector4<f32>,
    pub rotation: Quaternion<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SystemAction {
    Quit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CharacterAction {
    Motion(MotionInput),
}

/*
* Top level input enum that is the public API of the Input system
*/
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    System(SystemAction),
    Character(CharacterAction),
}
