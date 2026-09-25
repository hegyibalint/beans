package demo;

interface Labelled {}

class Box<T> {
    class Slot<U> {}
}

class Example<T extends Labelled> extends Box<String> {
    Box<String>.Slot<Integer> nested;
    Box<? extends Number> numbers;
    T label;
    int[] counts;
}
