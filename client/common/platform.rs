extern crate input;
extern crate property_tree;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Trait for logging output
pub trait ILogger {
    fn Log(&self, level: LogLevel, message: &str);
}

pub trait ISystem {
    fn Sleep(&self, ms: i32);
}

/// Trait for input management
pub trait IInputManager {
    fn PollInput(&self, logger: &dyn ILogger) -> Vec<input::Input>;
}

/// Trait for rendering
pub trait IRenderer {
    fn Render(&self, properties: &property_tree::PropertyTree, logger: &dyn ILogger);
}

pub trait IScene {
    fn Init(&mut self, properties: &mut property_tree::PropertyTree);
    fn Tick(&mut self, properties: &mut property_tree::PropertyTree, input: &Vec<input::Input>);
    fn Destroy(&mut self);
}

/// Platform bundles all subsystem interfaces together
pub struct Platform {
    pub logger: &'static dyn ILogger,
    pub renderer: &'static dyn IRenderer,
    pub input_manager: &'static dyn IInputManager,
    pub system: &'static dyn ISystem,
}

/// Context bundles platform services with the property tree
/// for passing to subsystems
pub struct Context {
    pub platform: Platform,
    pub property_tree: property_tree::PropertyTree,
}

impl Context {
    pub fn new(p: Platform) -> Self {
        return Self {
            platform: p,
            property_tree: property_tree::PropertyTree::new(),
        };
    }

    //TODO(oleg): replace scene with some kind of scene manager object
    // that can dynamically load new scenews in
    pub fn Run(&mut self, scene: &mut dyn IScene) {
        scene.Init(&mut self.property_tree);
        loop {
            let inputs = self.platform.input_manager.PollInput(self.platform.logger);
            for input in &inputs {
                match input {
                    input::Input::System(input::SystemAction::Quit) => return,
                    _ => {}
                }
            }
            scene.Tick(&mut self.property_tree, &inputs);
            self.platform
                .renderer
                .Render(&self.property_tree, self.platform.logger);
            self.platform.system.Sleep(32);
        }
        scene.Destroy();
    }
}
