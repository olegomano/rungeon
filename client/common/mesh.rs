extern crate handle;

/// Placeholder metadata for a mesh resource.
///
/// Vertex layout, index width, and the eventual mesh/image library are still
/// undecided. This exists so callers can already store meshes behind
/// [`handle::handle_t<Mesh>`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MeshMeta {
    pub vertex_count: u32,
    pub index_count: u32,
}

/// Placeholder mesh resource backed by opaque binary blobs.
///
/// For now this only stores raw pointers plus counts. Once the mesh format and
/// image library are chosen, this type can grow real accessors and loaders.
#[derive(Debug)]
pub struct Mesh {
    pub meta: MeshMeta,
    vertices: *const u8,
    vertex_bytes: usize,
    indices: *const u8,
    index_bytes: usize,
}

impl Mesh {
    pub fn Meta(&self) -> MeshMeta {
        self.meta
    }

    pub fn Vertices(&self) -> *const u8 {
        self.vertices
    }

    pub fn VertexBytes(&self) -> usize {
        self.vertex_bytes
    }

    pub fn Indices(&self) -> *const u8 {
        self.indices
    }

    pub fn IndexBytes(&self) -> usize {
        self.index_bytes
    }

    /// Create a mesh from opaque vertex/index blobs and simple counts.
    ///
    /// This is intentionally minimal until the mesh format is finalized.
    pub fn FromBlob(
        vertices: &[u8],
        indices: &[u8],
        meta: MeshMeta,
    ) -> Self {
        Self {
            meta,
            vertices: vertices.as_ptr(),
            vertex_bytes: vertices.len(),
            indices: indices.as_ptr(),
            index_bytes: indices.len(),
        }
    }

    /// Create a mesh from compile-time embedded asset blobs.
    pub fn FromEmbeddedAsset(
        vertices: &'static [u8],
        indices: &'static [u8],
        meta: MeshMeta,
    ) -> Self {
        Self {
            meta,
            vertices: vertices.as_ptr(),
            vertex_bytes: vertices.len(),
            indices: indices.as_ptr(),
            index_bytes: indices.len(),
        }
    }
}
