// JLS §6.5.5.1.
//
// Every case here needs on-demand imports and the ambiguity rules that come with
// them, and none of that resolves yet; written out, they would be nine tests
// failing for one reason. The claims to make once it lands:
//
//  - a missing simple type is unresolvable
//  - two on-demand imports offering one simple name are ambiguous
//  - an explicit on-demand import can collide with `java.lang`
//  - two on-demand paths to one declaration are deduplicated
//  - a type import and a static import reaching one type are deduplicated
//  - two inherited member types sharing a name are ambiguous
//  - diamond paths to one member type are deduplicated
//
// Two more belong to code actions rather than to resolution, i.e. the tier that
// is allowed to see past the scope: an unimported accessible type is importable,
// and every accessible candidate is offered, not just the first.
