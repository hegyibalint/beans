package beans.fixture;

import java.io.Serializable;

public class Feature implements Serializable {
    public int[] values;

    public Object[][] combine(String input, long amount) {
        return null;
    }

    public class Member {}

    public Object local() {
        class Local {}
        return new Local();
    }
}

record Point(int x, String name) {}

@interface Marker {}

enum Mode {
    ONE
}

interface Contract {}
