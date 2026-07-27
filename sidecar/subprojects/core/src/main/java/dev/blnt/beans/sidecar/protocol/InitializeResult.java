package dev.blnt.beans.sidecar.protocol;

import java.util.List;

/** Handshake reply: protocol version, host JVM facts, and the adapters found. */
public record InitializeResult(
        int protocolVersion, String javaHome, String javaVersion, List<String> adapters) {

    public static final int PROTOCOL_VERSION = 1;

    public static InitializeResult of(List<String> adapters) {
        return new InitializeResult(
                PROTOCOL_VERSION,
                System.getProperty("java.home"),
                System.getProperty("java.version"),
                adapters);
    }
}
