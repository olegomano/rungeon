extern crate handle;
extern crate transform;
extern crate image;

pub struct Sprite {
    pub image : handle::handle_t<image::Image>,
    pub transform : transform::Transform,
}

pub struct Mesh {}

pub struct Camera {}

pub struct LightSource {}

pub struct Particle {}

pub struct HeightMap {}


pub trait IRenderer{
    fn AddSprite(sprite : Sprite) -> handle::handle_t<Sprite>;
    fn AddMesh(mesh : Mesh) -> handle::handle_t<Mesh>;
    fn AddCamera(camera : Camera) -> handle::handle_t<Camera>;
    fn AddLightSource(light_source : LightSource) -> handle::handle_t<LightSource>;
    fn AddParticle(particle : Particle) -> handle::handle_t<Particle>;
    fn AddHeightMap(height_map : HeightMap) -> handle::handle_t<HeightMap>;

    fn RemoveSprite(sprite : handle::handle_t<Sprite>);
    fn RemoveMesh(mesh : handle::handle_t<Mesh>);
    fn RemoveCamera(camera : handle::handle_t<Camera>);
    fn RemoveLightSource(light_source : handle::handle_t<LightSource>);
    fn RemoveParticle(particle : handle::handle_t<Particle>);
    fn RemoveHeightMap(height_map : handle::handle_t<HeightMap>);
}