use beans_lang_java_model::File;

// Resolves a `Resolvable` by following [JLS $6.5](https://docs.oracle.com/javase/specs/jls/se21/html/jls-6.html#jls-6.5) (and others concerns like §6.6 (accessibility))
//
pub fn resolve(_file: &File, _at: usize) {
    // 1. Classify by context (§6.5.1).
    //
    // Seven categories, decided by where the name sits. No lookup happens
    // here.

    // 2. Reclassify (§6.5.2, §6.5.4).
    //
    // Only AmbiguousName and PackageOrTypeName. Variable beats type beats
    // package (§6.4.2), and "package" is answered without checking one
    // exists. A qualified name classifies its prefix first, recursively.

    // 3. ModuleName, PackageName (§6.5.3).
    //
    // §6.3: a subpackage is never in scope, `java` always is.

    // 4. Simple TypeName (§6.5.5.1).
    //
    // Every declaration of the name in scope here (§6.3), minus the shadowed
    // (§6.4.1), must leave exactly one. A type parameter then has static
    // context and inner class conditions (§8.1.3).

    // 5. Qualified TypeName (§6.5.5.2).
    //
    // Exactly one accessible member type of Q. Not a member, not accessible,
    // and more than one are three separate errors.

    // 6. Simple ExpressionName (§6.5.6.1).
    //
    // Exactly one local, parameter, exception parameter or field in scope at
    // this point. Then static context, inner class chain, and effective
    // finality (§4.12.4). Enum constants in a case label are a special case.

    // 7. Qualified ExpressionName (§6.5.6.2).
    //
    // A package prefix is an error. A type prefix wants a static field. An
    // expression prefix needs the type of that expression, which is §15.

    // 8. MethodName (§6.5.7.1, §15.12.1).
    //
    // §15.12.1 picks the type to search, in six cases. Unqualified uses the
    // comb rule: a nested class's supertypes before the enclosing class.
    // Overload selection is §15.12.2 and needs argument types.

    // 9. Accessibility (§6.6.1).
    //
    // Part of steps 5, 7 and 8, not a filter after them. `private` reaches
    // the enclosing *top level* type. `protected` needs §6.6.2, which needs
    // supertypes.

    // 10. Answer.
    //
    // Resolved, not found, ambiguous, inaccessible, wrong staticness. A
    // rejected candidate must not hide a later answer, and must survive to
    // the diagnostic.
}
