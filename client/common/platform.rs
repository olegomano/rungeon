extern crate input;
extern crate property_tree;

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

/// Trait for input management
pub trait IInputManager {
    fn PollInput(&self, logger: &dyn ILogger) -> Vec<input::Input>;
}

/// Trait for rendering
pub trait IRenderer {
    fn Render(&self, properties: &property_tree::PropertyTree, logger: &dyn ILogger);
}

/// Platform bundles all subsystem interfaces together
pub struct Platform {
    pub logger: &'static dyn ILogger,
    pub renderer: &'static dyn IRenderer,
    pub input_manager: &'static dyn IInputManager,
}

/// Context bundles platform services with the property tree
/// for passing to subsystems
pub struct Context {
    pub platform: Platform,
    pub property_tree: property_tree::PropertyTree,
}

pub trait IScene {
    fn Init(&mut self);
    fn Tick(&mut self, properties: &mut property_tree::PropertyTree);
    fn Destroy(&mut self);
}
