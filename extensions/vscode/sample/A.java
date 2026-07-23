class A {
    B field;

    void use(B other) {
        int v = other.value;
        other.greet();
    }
}
