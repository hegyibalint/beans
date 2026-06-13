package dev.blnt.beans.sidecar.protocol;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * The {@code initialize} handshake reply: protocol version, host JVM
 * facts, and the per-duty capability report (ADR-0031 — duties degrade
 * individually; a future {@code ap/run} is present only when the
 * sidecar runs on a JDK).
 */
public record InitializeResult(
        int protocolVersion, String javaHome, String javaVersion, Capabilities capabilities) {

    public record Capabilities(@JsonProperty("gradle/import") boolean gradleImport) {}

    public static InitializeResult current() {
        return new InitializeResult(
                1,
                System.getProperty("java.home"),
                System.getProperty("java.version"),
                new Capabilities(true));
    }
}
