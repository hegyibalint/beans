// JLS §§7.5.5 and 6.4.1.
//
// Everything here is out of reach: the fixture has no module roots, and module
// imports are not resolved. Two cases used to sit below, both of the form "X
// shadows a module import", and both passed only because the module import
// contributed nothing; a loser that does not exist cannot lose.
//
// The claims to make once module roots and §7.5.5 land:
//
//  - a module import provides an exported public type
//  - it includes the packages exported by transitively read modules
//  - one module import can introduce an ambiguous simple name
//  - a type import on demand shadows a module import
//  - a static import on demand shadows a module import
//  - the implicit `java.lang` import shadows a module import
//  - importing a module we do not read is an error
