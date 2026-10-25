package demo;

import types.ExternalImportedType;
import types.GenericType;
import types.TypeParameters;

class Demo {
    // Hover a primitive type.
    // Hovering `int` should show `int`.
    int primitive;
    // Hover a primitive array.
    // Hovering `int` should show `int[]`.
    int[] primitiveArray;
    // Navigate to a class in this file.
    // Go to Definition should land on `LocalType` below.
    LocalType local;
    // Navigate to a class in another file.
    // Go to Definition should open `types/ExternalImportedType.java`.
    ExternalImportedType imported;
    // Report an unresolved type.
    // The editor should underline `MissingType` as unresolved.
    MissingType missing;

    // Navigate a generic type and its type argument.
    // Each type name should navigate to its own declaration.
    GenericType<LocalType> generic;
    // Hover an upper-bounded wildcard.
    // Hovering `LocalType` should show `demo.LocalType` and its source.
    GenericType<? extends LocalType> upperBound;
    // Navigate to bounded type parameters in another file.
    // Go to Definition should open `types/TypeParameters.java`.
    TypeParameters<?> typeParameters;

    // Navigate a parameterized member type.
    // Go to Definition on `MemberType` should land on the nested class.
    GenericType<LocalType>.MemberType<ExternalImportedType> member;
    // Hover arrays inside and outside type arguments.
    // Hovering either type name should show its declaration and source.
    GenericType<TypeParameters<?>[]>[] nestedArrays;
    // Report an unresolved array element in a type argument.
    // The editor should underline only `MissingType`, not the enclosing type.
    GenericType<MissingType[]> unresolvedArray;
}

class LocalType {}
