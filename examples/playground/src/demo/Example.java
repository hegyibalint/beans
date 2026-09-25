package demo;

interface Labelled {}

class Box<T> {
    class Slot<U> {}
}

class Navigation {
    class Destination {}
    Destination next;
}

class Example<T extends Labelled> extends Box<String> {
    Box<String>.Slot<Integer> nested;
    Box<? extends Number> numbers;
    T label;
    int[] counts;
}
