extern crate nalgebra;
use nalgebra::{Matrix4, Point3, Quaternion, Unit, UnitQuaternion, Vector3};

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub matrix: Matrix4<f32>,
}

impl Transform {
    pub fn Identity() -> Self {
        Self {
            matrix: Matrix4::identity(),
        }
    }

    pub fn FromTranslationRotation(
        translation: Vector3<f32>,
        rotation: UnitQuaternion<f32>,
    ) -> Self {
        let rot = rotation.to_rotation_matrix().to_homogeneous();
        let mut matrix = rot;
        matrix.fixed_slice_mut::<3, 1>(0, 3).copy_from(&translation);
        Self { matrix }
    }

    pub fn LookAt(from: Point3<f32>, to: Point3<f32>, up: Vector3<f32>) -> Self {
        let f = (to - from).normalize();
        let s = f.cross(&up).normalize();
        let u = s.cross(&f);
        let p = from.coords;

        let matrix = Matrix4::new(
            s.x, u.x, -f.x, p.x, s.y, u.y, -f.y, p.y, s.z, u.z, -f.z, p.z, 0.0, 0.0, 0.0, 1.0,
        );
        Self { matrix }
    }

    pub fn ToTranslation(&self) -> Vector3<f32> {
        self.matrix.fixed_slice::<3, 1>(0, 3).into()
    }

    pub fn ToRotation(&self) -> UnitQuaternion<f32> {
        UnitQuaternion::from_matrix(&self.matrix.fixed_slice::<3, 3>(0, 0).into())
    }

    pub fn Lerp(a: &Self, b: &Self, t: f32) -> Self {
        let ta = a.ToTranslation();
        let tb = b.ToTranslation();
        let interp_translation = ta.lerp(&tb, t);

        let ra = a.ToRotation();
        let rb = b.ToRotation();
        let interp_rotation = ra.slerp(&rb, t);

        Self::FromTranslationRotation(interp_translation, interp_rotation)
    }

    pub fn RotateAroundPoint(point: Point3<f32>, angle_radians: f32) -> Self {
        let rot = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), angle_radians);
        let t = point.coords;
        let neg_t = -t;

        let translate_to_origin = Matrix4::new_translation(&neg_t);
        let rotation = rot.to_homogeneous();
        let translate_back = Matrix4::new_translation(&t);

        Self {
            matrix: translate_back * rotation * translate_to_origin,
        }
    }

    pub fn ApplyTranslation(&mut self, offset: Vector3<f32>) {
        let translation = Matrix4::new_translation(&offset);
        self.matrix = translation * self.matrix;
    }

    pub fn TransformPoint(&self, p: &Point3<f32>) -> Point3<f32> {
        self.matrix.transform_point(p)
    }

    pub fn Inverse(&self) -> Option<Self> {
        self.matrix.try_inverse().map(|m| Self { matrix: m })
    }

    pub fn Concat(&self, other: &Self) -> Self {
        Self {
            matrix: self.matrix * other.matrix,
        }
    }
}
