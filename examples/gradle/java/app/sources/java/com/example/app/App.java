package com.example.app;

import com.example.lib.BuildInfo;
import com.example.lib.Greeting;
import org.apache.commons.lang3.StringUtils;

public final class App {

    public static String describe(String name) {
        return new Greeting().greet(StringUtils.capitalize(name)) + " (v" + BuildInfo.VERSION + ")";
    }

    public static void main(String[] args) {
        System.out.println(describe("beans"));
    }
}
