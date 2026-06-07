//! Lightweight primitives library
//!
//! Exposes packed vertex formats suitable for uploading directly to Vulkan vertex
//! buffers. Vertex layout is 4 floats for position (x, y, z, w) followed by 2
//! floats for texture coordinates (u, v). The layout is #[repr(C)] so it is
//! compatible with C layout and can be safely transmuted to bytes for GPU
//! uploads (using the provided helper).

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    /// Position as vec4 (x, y, z, w). Use w = 1.0 for positional vertices.
    pub pos: [f32; 4],
    /// Texture coordinates (u, v).
    pub uv: [f32; 2],
}

impl Vertex {
    /// Create a new vertex with explicit components.
    pub const fn new(x: f32, y: f32, z: f32, w: f32, u: f32, v: f32) -> Self {
        Self {
            pos: [x, y, z, w],
            uv: [u, v],
        }
    }

    /// Return the supplied slice of vertices as a byte slice for buffer uploads.
    ///
    /// Safety: implemented with a safe API but using an unsafe conversion under
    /// the hood. The memory layout is #[repr(C)] and contains only floats so
    /// this is safe in our usage.
    pub fn as_bytes(slice: &[Vertex]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                slice.as_ptr() as *const u8,
                slice.len() * std::mem::size_of::<Vertex>(),
            )
        }
    }
}

/// Simple triangle (3 vertices) with packed pos(x,y,z,w) and uv(u,v).
pub const TRIANGLE: [Vertex; 3] = [
    // position (x,y,z,w), uv
    Vertex::new(0.0, -0.5, 0.0, 1.0, 0.5, 1.0),
    Vertex::new(0.5, 0.5, 0.0, 1.0, 1.0, 0.0),
    Vertex::new(-0.5, 0.5, 0.0, 1.0, 0.0, 0.0),
];

/// Axis-aligned quad defined as 4 unique vertices (use indices to draw as two
/// triangles). Positions are centered at origin, size 1x1.
pub const QUAD_VERTS: [Vertex; 4] = [
    Vertex::new(-0.5, -0.5, 0.0, 1.0, 0.0, 1.0),
    Vertex::new(0.5, -0.5, 0.0, 1.0, 1.0, 1.0),
    Vertex::new(0.5, 0.5, 0.0, 1.0, 1.0, 0.0),
    Vertex::new(-0.5, 0.5, 0.0, 1.0, 0.0, 0.0),
];

/// Quad expanded into 6 vertices (triangle list) for APIs that don't use
/// indices or when you want a contiguous vertex stream.
pub const QUAD_TRIANGLES: [Vertex; 6] = [
    QUAD_VERTS[0],
    QUAD_VERTS[1],
    QUAD_VERTS[2],
    QUAD_VERTS[2],
    QUAD_VERTS[3],
    QUAD_VERTS[0],
];

/// Fullscreen triangle which covers the entire clip-space. Useful for
/// post-processing or screen-space passes. Coordinates are already in clip
/// space (no vertex shader projection needed) — set your pipeline accordingly.
pub const FULLSCREEN_TRIANGLE: [Vertex; 3] = [
    // These positions are common trick to cover full screen with one triangle
    Vertex::new(-1.0, -1.0, 0.0, 1.0, 0.0, 0.0),
    Vertex::new(3.0, -1.0, 0.0, 1.0, 2.0, 0.0),
    Vertex::new(-1.0, 3.0, 0.0, 1.0, 0.0, 2.0),
];

/// Unit square (0..1) at z=0 useful for texture-space rendering.
pub const UNIT_SQUARE: [Vertex; 4] = [
    Vertex::new(0.0, 0.0, 0.0, 1.0, 0.0, 1.0),
    Vertex::new(1.0, 0.0, 0.0, 1.0, 1.0, 1.0),
    Vertex::new(1.0, 1.0, 0.0, 1.0, 1.0, 0.0),
    Vertex::new(0.0, 1.0, 0.0, 1.0, 0.0, 0.0),
];

/// Circle approximation (triangle fan) centered at origin. Provides a small
/// utility to generate a circle at compile-time for a given resolution.
pub fn circle_vertices(resolution: usize) -> Vec<Vertex> {
    // Dynamic circle generator. Returns a triangle-fan layout: center followed
    // by 'resolution' perimeter points. If resolution == 0, a default of 16
    // segments is used.
    let res = if resolution == 0 { 16 } else { resolution };
    let mut verts = Vec::with_capacity(res + 1);
    verts.push(Vertex::new(0.0, 0.0, 0.0, 1.0, 0.5, 0.5)); // center
    for i in 0..res {
        let theta = 2.0 * std::f32::consts::PI * (i as f32) / (res as f32);
        let x = theta.cos() * 0.5;
        let y = theta.sin() * 0.5;
        // map uv such that center is 0.5,0.5 and perimeter maps to [0..1]
        verts.push(Vertex::new(x, y, 0.0, 1.0, x + 0.5, 1.0 - (y + 0.5)));
    }
    verts
}

/// Cube (24 unique vertices) with per-face UVs. This layout avoids sharing
/// vertices between faces so each face can have its own UVs without seams.
/// The cube is centered at origin with side length 1 (extents +/-0.5).
pub const CUBE_VERTS: [Vertex; 24] = [
    // +X face
    Vertex::new(0.5, -0.5, -0.5, 1.0, 0.0, 1.0),
    Vertex::new(0.5, -0.5, 0.5, 1.0, 1.0, 1.0),
    Vertex::new(0.5, 0.5, 0.5, 1.0, 1.0, 0.0),
    Vertex::new(0.5, 0.5, -0.5, 1.0, 0.0, 0.0),
    // -X face
    Vertex::new(-0.5, -0.5, 0.5, 1.0, 0.0, 1.0),
    Vertex::new(-0.5, -0.5, -0.5, 1.0, 1.0, 1.0),
    Vertex::new(-0.5, 0.5, -0.5, 1.0, 1.0, 0.0),
    Vertex::new(-0.5, 0.5, 0.5, 1.0, 0.0, 0.0),
    // +Y face
    Vertex::new(-0.5, 0.5, -0.5, 1.0, 0.0, 1.0),
    Vertex::new(0.5, 0.5, -0.5, 1.0, 1.0, 1.0),
    Vertex::new(0.5, 0.5, 0.5, 1.0, 1.0, 0.0),
    Vertex::new(-0.5, 0.5, 0.5, 1.0, 0.0, 0.0),
    // -Y face
    Vertex::new(-0.5, -0.5, 0.5, 1.0, 0.0, 1.0),
    Vertex::new(0.5, -0.5, 0.5, 1.0, 1.0, 1.0),
    Vertex::new(0.5, -0.5, -0.5, 1.0, 1.0, 0.0),
    Vertex::new(-0.5, -0.5, -0.5, 1.0, 0.0, 0.0),
    // +Z face
    Vertex::new(-0.5, -0.5, 0.5, 1.0, 0.0, 1.0),
    Vertex::new(-0.5, 0.5, 0.5, 1.0, 1.0, 1.0),
    Vertex::new(0.5, 0.5, 0.5, 1.0, 1.0, 0.0),
    Vertex::new(0.5, -0.5, 0.5, 1.0, 0.0, 0.0),
    // -Z face
    Vertex::new(0.5, -0.5, -0.5, 1.0, 0.0, 1.0),
    Vertex::new(0.5, 0.5, -0.5, 1.0, 1.0, 1.0),
    Vertex::new(-0.5, 0.5, -0.5, 1.0, 1.0, 0.0),
    Vertex::new(-0.5, -0.5, -0.5, 1.0, 0.0, 0.0),
];

/// Indices to draw the cube as triangles (6 faces * 2 triangles * 3 indices).
/// Cube expanded into a triangle-list (36 vertices). This is convenient for
/// APIs or pipelines that prefer non-indexed draws.
pub const CUBE_TRIANGLES: [Vertex; 36] = [
    // +X
    CUBE_VERTS[0],
    CUBE_VERTS[1],
    CUBE_VERTS[2],
    CUBE_VERTS[2],
    CUBE_VERTS[3],
    CUBE_VERTS[0],
    // -X
    CUBE_VERTS[4],
    CUBE_VERTS[5],
    CUBE_VERTS[6],
    CUBE_VERTS[6],
    CUBE_VERTS[7],
    CUBE_VERTS[4],
    // +Y
    CUBE_VERTS[8],
    CUBE_VERTS[9],
    CUBE_VERTS[10],
    CUBE_VERTS[10],
    CUBE_VERTS[11],
    CUBE_VERTS[8],
    // -Y
    CUBE_VERTS[12],
    CUBE_VERTS[13],
    CUBE_VERTS[14],
    CUBE_VERTS[14],
    CUBE_VERTS[15],
    CUBE_VERTS[12],
    // +Z
    CUBE_VERTS[16],
    CUBE_VERTS[17],
    CUBE_VERTS[18],
    CUBE_VERTS[18],
    CUBE_VERTS[19],
    CUBE_VERTS[16],
    // -Z
    CUBE_VERTS[20],
    CUBE_VERTS[21],
    CUBE_VERTS[22],
    CUBE_VERTS[22],
    CUBE_VERTS[23],
    CUBE_VERTS[20],
];

/// Generates an indexed UV-sphere (latitude/longitude). Returns (vertices, indices).
///
/// - `radius`: sphere radius
/// - `lat_segments`: number of latitude segments (>= 2)
/// - `lon_segments`: number of longitude segments (>= 3)
pub fn uv_sphere(radius: f32, lat_segments: usize, lon_segments: usize) -> Vec<Vertex> {
    let lat = lat_segments.max(2);
    let lon = lon_segments.max(3);

    let mut verts: Vec<Vertex> = Vec::with_capacity((lat + 1) * (lon + 1));
    for y in 0..=lat {
        let v = y as f32 / lat as f32; // 0..1
        let theta = v * std::f32::consts::PI; // 0..PI
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for x in 0..=lon {
            let u = x as f32 / lon as f32; // 0..1
            let phi = u * 2.0 * std::f32::consts::PI; // 0..2PI
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let px = radius * sin_theta * cos_phi;
            let py = radius * cos_theta;
            let pz = radius * sin_theta * sin_phi;

            verts.push(Vertex::new(px, py, pz, 1.0, u, 1.0 - v));
        }
    }

    // Convert the regular grid into a triangle-list (non-indexed). For each
    // quad on the parameterization we emit two triangles (a,b,a+1) and
    // (a+1,b,b+1) where a and b are grid indices.
    let mut triangles: Vec<Vertex> = Vec::with_capacity(lat * lon * 6);
    for y in 0..lat {
        for x in 0..lon {
            let a = y * (lon + 1) + x;
            let b = (y + 1) * (lon + 1) + x;

            // triangle 1: a, b, a+1
            triangles.push(verts[a].clone());
            triangles.push(verts[b].clone());
            triangles.push(verts[a + 1].clone());

            // triangle 2: a+1, b, b+1
            triangles.push(verts[a + 1].clone());
            triangles.push(verts[b].clone());
            triangles.push(verts[b + 1].clone());
        }
    }

    triangles
}

// End of primitives library
