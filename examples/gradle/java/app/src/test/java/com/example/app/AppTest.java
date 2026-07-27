package com.example.app;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertTrue;

class AppTest {

    @Test
    void describes_with_the_generated_version() {
        assertTrue(App.describe("beans").endsWith("(v1.0)"));
    }
}
