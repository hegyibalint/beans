package beans.fixture;

import java.io.Serializable;

public class Feature implements Serializable {
    public int[] values;
    protected int guarded;
    int shared;
    private int hidden;

    public Object[][] combine(String input, long amount) {
        return null;
    }

    protected void guard() {}

    void share() {}

    private void hide() {}

    public class Member {}

    protected class Guarded {}

    class Shared {}

    private class Hidden {}

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
