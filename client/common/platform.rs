extern crate input;
extern crate nalgebra;
use nalgebra::{Matrix4, Point3, Quaternion, Unit, UnitQuaternion, Vector3, Vector4};
use std::time::{Duration, Instant};

pub trait GameState {
    fn New() -> Self;
}

pub trait Renderer<G: GameState> {
    fn DoRender(&mut self, state: &G);
}

pub trait Input {
    fn GetInput(&mut self) -> Vec<input::Input>;
}

pub trait Scene<G: GameState> {
    fn HandleInput(&mut self, state: &mut G, input: &input::CharacterAction);
    fn Tick(&mut self, state: &mut G);
}

pub struct Params {
    pub frame_rate: u32,
}

pub struct Platform<G: GameState, R: Renderer<G>, S: Scene<G>, I: Input> {
    renderer: R,
    input: I,
    game_state: G,
    scene: S,
    params: Params,
}

impl<G: GameState, R: Renderer<G>, S: Scene<G>, I: Input> Platform<G, R, S, I> {
    pub fn Create(r: R, s: S, i: I, p: Params) -> Self {
        return Self {
            renderer: r,
            input: i,
            scene: s,
            game_state: G::New(),
            params: p,
        };
    }

    pub fn Run(&mut self) {
        const FRAME_DURATION: Duration = Duration::from_micros(16_666); // ~60Hz
        loop {
            let start = Instant::now();
            if !self.Tick() {
                break;
            }
            let elapsed = start.elapsed();
            if elapsed < FRAME_DURATION {
                std::thread::sleep(FRAME_DURATION - elapsed);
            }
        }
    }

    fn Tick(&mut self) -> bool {
        let inputs = self.input.GetInput();
        for input in inputs {
            match input {
                input::Input::System(action) => match action {
                    input::SystemAction::Quit => {
                        return false;
                    }
                    _ => {}
                },
                input::Input::Character(action) => {
                    self.scene.HandleInput(&mut self.game_state, &action);
                }
                _ => {}
            }
        }
        self.scene.Tick(&mut self.game_state);
        self.renderer.DoRender(&self.game_state);
        return true;
    }
}
