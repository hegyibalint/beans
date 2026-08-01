package com.example.b;

import com.example.a.AMain;
import com.example.a.ATest;

/**
 * Unit b-main, which depends on a-main.
 *
 * The interesting method is toOtherMain: the edge exists, so every name
 * resolves and F12 lands in project a. Whether this place may touch what it
 * found is a different question, and the package boundary answers most of it.
 */
class BMain {
    public int open;
    int shared;
    private int hidden;

    /** b-main -> b-main. */
    void toOwnUnit(BMain target) {
        int a = target.open;    // GOOD
        int b = target.shared;  // GOOD
        int c = target.hidden;  // GOOD, same top level class
    }

    /** b-main -> a-main. The edge beans.toml declares. */
    void toOtherMain(AMain target) {
        int a = target.open;    // GOOD, public
        int b = target.shared;  // BAD, squiggle: package-private in com.example.a
        int c = target.hidden;  // BAD, squiggle: private in AMain
    }

    /** b-main -> b-test. Its own tests, and still no. */
    void toOwnTests(BTest target) {
        int a = target.open;    // BAD
    }

    /** b-main -> a-test. The other project's tests. */
    void toOtherTests(ATest target) {
        int a = target.open;    // BAD
    }
}
