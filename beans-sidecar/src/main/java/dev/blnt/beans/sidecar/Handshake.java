package dev.blnt.beans.sidecar;

import com.google.gson.JsonObject;

/**
 * The initialize handshake: protocol version plus per-duty capability
 * report (ADR-0031). Duties degrade individually — e.g. a future
 * {@code ap/run} requires running on a JDK; {@code gradle/import}
 * works anywhere.
 */
final class Handshake {

    static JsonObject capabilities() {
        JsonObject caps = new JsonObject();
        caps.addProperty("gradle/import", true);
        // ap/run will require ToolProvider.getSystemJavaCompiler() != null.

        JsonObject result = new JsonObject();
        result.addProperty("protocolVersion", 1);
        result.addProperty("javaHome", System.getProperty("java.home"));
        result.addProperty("javaVersion", System.getProperty("java.version"));
        result.add("capabilities", caps);
        return result;
    }

    private Handshake() {}
}
