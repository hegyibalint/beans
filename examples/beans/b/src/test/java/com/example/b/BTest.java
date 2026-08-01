package com.example.b;

import com.example.a.AMain;
import com.example.a.ATest;

/**
 * Unit b-test, which depends on b-main and on a-main.
 *
 * The widest scope in the example, and it still cannot see a single test class
 * other than its own.
 */
class BTest {
    public int open;
    int shared;
    private int hidden;

    /** b-test -> b-test. */
    void toOwnUnit(BTest target) {
        int a = target.open;    // GOOD
        int b = target.shared;  // GOOD
        int c = target.hidden;  // GOOD, same top level class
    }

    /** b-test -> b-main. Same package, so package-private is fine. */
    void toOwnMain(BMain target) {
        int a = target.open;    // GOOD
        int b = target.shared;  // GOOD, both are com.example.b
        int c = target.hidden;  // BAD, squiggle: private stays inside BMain
    }

    /**
     * b-test -> a-main, and only because beans.toml names a-main here.
     * Delete it from unit.b-test and this whole method goes dark, even though
     * b-main keeps its own edge: depends_on does not chain.
     */
    void toOtherMain(AMain target) {
        int a = target.open;    // GOOD
        int b = target.shared;  // BAD, squiggle: package-private in com.example.a
        int c = target.hidden;  // BAD, squiggle: private in AMain
    }

    /** b-test -> a-test. Cross-project tests never see each other. */
    void toOtherTests(ATest target) {
        int a = target.open;    // BAD
    }
}
