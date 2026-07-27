package com.example.lib;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class GreetingTest {

    @Test
    void greets_by_name() {
        assertEquals("Hello, world!", new Greeting().greet("world"));
    }
}
