/*
* A Resource is a file or peice of memory that the server has access to but the client does not
* The Client must dynamically load this
*/
pub struct Resource {}
/*
* A Handle is something both the client and the server know about
*/
pub struct Handle {}

/*
* A peice of data whose lifecycle is dynamically managed by the server
* this would include things like players, NPCs, items, etc
* This is a unique thing in the game world
*/
pub struct Entity {}
