package demo;

interface Labelled {}

class Box<T> {
    class Slot<U> {}
}

// A single source file is enough to explore the hover positions Beans models today.
class Showcase<T extends Labelled> extends Box<String> {
    Box<String>.Slot<Integer> nested;
    Box<? extends Number> numbers;
    T label;
    int[] counts;
}
