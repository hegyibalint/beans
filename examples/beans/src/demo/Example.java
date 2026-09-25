package demo;

class Base<T> {}

class Outer<T> {
    class Inner<U> {}
}

class Example extends Base<Outer<String>.Inner<Integer>> {
    Outer<String>.Inner<Integer> member;
    int[] counts;
}
