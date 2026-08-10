extern crate handle;
extern crate image;
extern crate sparse_buffer;
extern crate transform;

pub struct Sprite {
    pub image: handle::handle_t<image::Image>,
    pub transform: transform::Transform,
}

pub struct Mesh {}

pub struct Camera {}

pub struct LightSource {}

pub struct Particle {}

pub struct HeightMap {}

pub enum ChangeType {
    Add,
    Delete,
    Mutate,
}

pub enum RenderObject {
    Sprite(handle::handle_t<Sprite>),
    Mesh(handle::handle_t<Mesh>),
    Camera(handle::handle_t<Camera>),
    LightSource(handle::handle_t<LightSource>),
    Particle(handle::handle_t<Particle>),
    HeightMap(handle::handle_t<HeightMap>),
}

pub struct RenderState {
    sprites: sparse_buffer::SparseBuffer<Sprite>,
    meshes: sparse_buffer::SparseBuffer<Mesh>,
    cameras: sparse_buffer::SparseBuffer<Camera>,
    light_sources: sparse_buffer::SparseBuffer<LightSource>,
    particles: sparse_buffer::SparseBuffer<Particle>,
    height_maps: sparse_buffer::SparseBuffer<HeightMap>,
    changes: Vec<(RenderObject, ChangeType)>,
}

pub trait IRenderer{
    fn Render(&mut self, &mut state : RenderState)
}

impl RenderState {
    pub fn New() -> Self {
        RenderState {
            sprites: sparse_buffer::SparseBuffer::New(),
            meshes: sparse_buffer::SparseBuffer::New(),
            cameras: sparse_buffer::SparseBuffer::New(),
            light_sources: sparse_buffer::SparseBuffer::New(),
            particles: sparse_buffer::SparseBuffer::New(),
            height_maps: sparse_buffer::SparseBuffer::New(),
            changes: Vec::new(),
        }
    }

    pub fn AddSprite(&mut self, sprite: Sprite) -> handle::handle_t<Sprite> {
        let handle = self.sprites.Allocate(sprite);
        self.changes.push((RenderObject::Sprite(handle), ChangeType::Add));
        handle
    }

    pub fn AddMesh(&mut self, mesh: Mesh) -> handle::handle_t<Mesh> {
        let handle = self.meshes.Allocate(mesh);
        self.changes.push((RenderObject::Mesh(handle), ChangeType::Add));
        handle
    }

    pub fn AddCamera(&mut self, camera: Camera) -> handle::handle_t<Camera> {
        let handle = self.cameras.Allocate(camera);
        self.changes.push((RenderObject::Camera(handle), ChangeType::Add));
        handle
    }

    pub fn AddLightSource(&mut self, light_source: LightSource) -> handle::handle_t<LightSource> {
        let handle = self.light_sources.Allocate(light_source);
        self.changes.push((RenderObject::LightSource(handle), ChangeType::Add));
        handle
    }

    pub fn AddParticle(&mut self, particle: Particle) -> handle::handle_t<Particle> {
        let handle = self.particles.Allocate(particle);
        self.changes.push((RenderObject::Particle(handle), ChangeType::Add));
        handle
    }

    pub fn AddHeightMap(&mut self, height_map: HeightMap) -> handle::handle_t<HeightMap> {
        let handle = self.height_maps.Allocate(height_map);
        self.changes.push((RenderObject::HeightMap(handle), ChangeType::Add));
        handle
    }

    pub fn MutateSprite(
        &mut self,
        sprite_handle: handle::handle_t<Sprite>,
        mutator: impl FnOnce(&mut Sprite),
    ) {
        if let Some(sprite) = self.sprites.GetMut(sprite_handle) {
            mutator(sprite);
            self.changes.push((RenderObject::Sprite(sprite_handle), ChangeType::Mutate));
        }
    }

    pub fn RemoveSprite(&mut self, sprite_handle: handle::handle_t<Sprite>) {
        self.sprites.Free(sprite_handle);
        self.changes.push((RenderObject::Sprite(sprite_handle), ChangeType::Delete));
    }

    pub fn RemoveMesh(&mut self, mesh_handle: handle::handle_t<Mesh>) {
        self.meshes.Free(mesh_handle);
        self.changes.push((RenderObject::Mesh(mesh_handle), ChangeType::Delete));
    }

    pub fn RemoveCamera(&mut self, camera_handle: handle::handle_t<Camera>) {
        self.cameras.Free(camera_handle);
        self.changes.push((RenderObject::Camera(camera_handle), ChangeType::Delete));
    }

    pub fn RemoveLightSource(&mut self, light_source_handle: handle::handle_t<LightSource>) {
        self.light_sources.Free(light_source_handle);
        self.changes.push((RenderObject::LightSource(light_source_handle), ChangeType::Delete));
    }

    pub fn RemoveParticle(&mut self, particle_handle: handle::handle_t<Particle>) {
        self.particles.Free(particle_handle);
        self.changes.push((RenderObject::Particle(particle_handle), ChangeType::Delete));
    }

    pub fn RemoveHeightMap(&mut self, height_map_handle: handle::handle_t<HeightMap>) {
        self.height_maps.Free(height_map_handle);
        self.changes.push((RenderObject::HeightMap(height_map_handle), ChangeType::Delete));
    }

    pub fn GetFrameChanges(&self) -> &[(RenderObject, ChangeType)] {
        &self.changes
    }

    pub fn ClearFrameChanges(&mut self) {
        self.changes.clear();
    }
