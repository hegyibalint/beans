class TypeArguments<A> {
    static class Box<T> {}
    static class Value {}

    Box<A> parameter;       // Box: line 2; A: type parameter on line 1.
    Box<Value> concrete;    // Box: line 2; Value: class on line 3.
    Box<Box<A>> nested;     // Both Boxes: line 2; A: type parameter on line 1.

    class Inner {
        class A {}

        // Inner.A (a class) shadows TypeArguments' parameter A: JLS §6.4.1.
        Box<A> shadowed;    // Box: line 2; A: class on line 10 wins over parameter on line 1.
    }
}
