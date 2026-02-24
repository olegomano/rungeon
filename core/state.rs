extern crate memory;
extern crate sparce_buffer;
extern crate transform;

/*
 * Represents the types for the in memory representation of the game state
 */

/*
* Represents entities that real players can modify
*/
pub struct Player {
    location: transform::Transform,
}

pub struct Npc {}

/*
* Represents the state that we want to synchronize between server and clients
* This is a collection of handle_t pointers into the real memory store
*/
pub struct WorldState {}
