package com.example.a;

import com.example.b.BMain;
import com.example.b.BTest;

/**
 * Unit a-main. It declares no depends_on, so it sees itself and nothing else.
 *
 * GOOD lines resolve. Put the cursor on a name, press F12, and you land on the
 * declaration, even in a file you never opened.
 *
 * BAD targets resolve to nothing, so F12 does nothing. Their type names carry
 * the scope diagnostic; member accesses through those parameters stay quiet.
 */
public class AMain {
    public int open;
    int shared;
    private int hidden;

    /** a-main -> a-main. A unit always sees itself. */
    void toOwnUnit(AMain target) {
        int a = target.open;    // GOOD
        int b = target.shared;  // GOOD
        int c = target.hidden;  // GOOD, we are inside the class that declares it
    }

    /**
     * a-main -> a-test. Same project, and ATest is in this very package
     * `com.example.a`. Still out of reach: nothing depends on a test unit.
     * The package is not what decides this, the unit is.
     */
    void toOwnTests(ATest target) {
        int a = target.open;    // BAD
    }

    /** a-main -> b-main. b depends on a, so a must not see b back. */
    void toOtherMain(BMain target) {
        int a = target.open;    // BAD
    }

    /** a-main -> b-test. Another project's tests. */
    void toOtherTests(BTest target) {
        int a = target.open;    // BAD
    }
}
