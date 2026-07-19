class Example {
    Runnable make() {
        return new Runnable() {
            class Member {}

            public void run() {}
        };
    }
}
