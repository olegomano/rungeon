extern crate input;
extern crate property;
extern crate property_tree;
extern crate transform;
extern crate platform as common_platform;

use common_platform::{IScene, LogLevel, ILogger};
use property::PropertyValue;
use transform::Transform;

pub struct TestScene {
    rect_entity: Option<property::ObjectId>,
    rect_property: Option<property::PropertyKey>,
    rect_position: nalgebra::Vector3<f32>,
    logger: Box<dyn ILogger>,
}

impl TestScene {
    pub fn new(logger: Box<dyn ILogger>) -> Self {
        TestScene {
            rect_entity: None,
            rect_property: None,
            rect_position: nalgebra::Vector3::new(400.0, 300.0, 0.0),
            logger,
        }
    }

    pub fn rect_position(&self) -> nalgebra::Vector3<f32> {
        self.rect_position
    }
}

impl IScene for TestScene {
    fn Init(&mut self, _properties: &mut property_tree::PropertyTree) {
        self.logger.Log(LogLevel::Info, "TestScene initialized - using property tree for state management");
    }

    fn Tick(&mut self, properties: &mut property_tree::PropertyTree, input: &Vec<input::Input>) {
        // Create entity on first tick if not already created
        if self.rect_entity.is_none() {
            let entity = properties.CreateObject();
            self.rect_entity = Some(entity);

            // Initialize transform property in the property tree
            let transform = PropertyValue::RendererTransform(Transform::Identity());
            let prop_key = properties.CreateProperty(entity, transform);
            self.rect_property = Some(prop_key);
            self.logger.Log(LogLevel::Info, "Created rectangle entity in property tree");
        }

        // Process input events - handle movement
        for evt in input {
            match evt {
                input::Input::Character(action) => {
                    match action {
                        input::CharacterAction::Motion(motion) => {
                            // Apply movement from arrow keys
                            self.rect_position.x += motion.movement.x;
                            self.rect_position.y += motion.movement.y;
                            self.logger.Log(LogLevel::Debug, &format!("Rect position updated to ({}, {})", 
                                self.rect_position.x, self.rect_position.y));
                        }
                    }
                }
                input::Input::System(action) => {
                    match action {
                        input::SystemAction::Quit => {
                            self.logger.Log(LogLevel::Info, "Quit action received");
                        }
                    }
                }
            }
        }

        // Update transform in property tree using saved property key
        if let (Some(_entity), Some(prop_key)) = (self.rect_entity, self.rect_property) {
            let transform = Transform::Identity();
            let prop = property::Property {
                key: prop_key,
                value: PropertyValue::RendererTransform(transform),
                version: property::Version::Null(),
            };
            properties.WriteProperty(prop);
        }
    }

    fn Destroy(&mut self) {
        self.logger.Log(LogLevel::Info, "TestScene destroyed");
    }
}
