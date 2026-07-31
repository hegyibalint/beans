// JLS §§7.5.3, 7.5.4, and 6.4.1.
//
// Static imports are not resolved, neither single nor on-demand, so all ten cases
// would fail for that one reason. The claims to make once they land:
//
//  - a single static import provides a static member type
//  - a static on-demand import provides a static member type
//  - a single static import rejects a non-static inner type
//  - a static on-demand import excludes a non-static inner type
//  - a static import can reach an inherited static member type
//  - importing a missing member is an error
//  - importing an inaccessible member is an error
//  - one single static import may expose ambiguous inherited types
//  - a type import and a static on-demand import of distinct types are ambiguous
//  - a duplicate static on-demand import is redundant
