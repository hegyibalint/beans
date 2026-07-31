// JLS §§6.5.2, 6.5.4, and 6.5.5.2.
//
// A qualified type reference is not resolved at all, so there is nothing here to
// be pending about yet; all eleven cases would fail for that one reason. The
// claims to make once it lands:
//
//  - a fully qualified top level type needs no import
//  - a fully qualified member type resolves
//  - an imported outer type can qualify a member type
//  - a qualified inherited member denotes the member of its declaring type
//  - a type parameter can qualify a member type of its bound
//  - a missing type in an existing package is unresolvable
//  - a missing member of an existing type is unresolvable
//  - an inaccessible qualified type is rejected, top level and member alike
//  - an in-scope type prefix obscures a same-named package
//  - a source member name maps to its JVM binary identity
