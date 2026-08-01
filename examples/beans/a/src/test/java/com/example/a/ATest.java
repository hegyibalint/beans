package com.example.a;

import com.example.b.BMain;
import com.example.b.BTest;

/**
 * Unit a-test, which depends on a-main.
 *
 * This file shows the second axis. Scoping decides whether a name resolves at
 * all; JLS 26 6.6.1 then decides whether this place may touch it. The two
 * answers are independent, and only the second one produces a squiggle.
 */
class ATest {
    public int open;
    int shared;
    private int hidden;

    /** a-test -> a-test. */
    void toOwnUnit(ATest target) {
        int a = target.open;    // GOOD
        int b = target.shared;  // GOOD
        int c = target.hidden;  // GOOD, same top level class
    }

    /** a-test -> a-main. The edge beans.toml declares. */
    void toOwnMain(AMain target) {
        int a = target.open;    // GOOD, public
        int b = target.shared;  // GOOD, package-private and we are in com.example.a
        int c = target.hidden;  // BAD, squiggle: private stays inside AMain
    }

    /** a-test -> b-main. a has no edge to b at all. */
    void toOtherMain(BMain target) {
        int a = target.open;    // BAD
    }

    /** a-test -> b-test. Tests never see other tests. */
    void toOtherTests(BTest target) {
        int a = target.open;    // BAD
    }
}
