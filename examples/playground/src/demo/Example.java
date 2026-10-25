package demo;

import library.Widget;

interface Labelled {}

class Box<T> {
    class Slot<U> {}
}

// Combine a bounded parameter with a generic superclass.
class Example<T extends Labelled> extends Box<String> {
    // Hover a primitive array.
    int[] counts;
    // Navigate to a type in another unit.
    Widget imported;
    // Navigate to the type parameter.
    T label;
    // Report an unresolved field type.
    Missing missing;

    // Hover an upper-bounded wildcard.
    Box<? extends Number> numbers;
    // Follow a generic type across files.
    Generic<?> generic;

    // Navigate a parameterized member type.
    Box<String>.Slot<Integer> nested;
    // Hover arrays inside and outside type arguments.
    Box<Generic<?>[]>[] batches;
    // Report an unresolved array element in a type argument.
    Box<Missing[]> collection;
}
