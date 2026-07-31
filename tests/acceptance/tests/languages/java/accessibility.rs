// JLS 26 §6.6.1 decides whether a name a file resolved is a name it may use.
// The two are separate questions here: resolution stays permissive so that
// navigation still reaches the declaration, and the diagnostic is what says it
// may not be touched.

mod package;
mod private;
