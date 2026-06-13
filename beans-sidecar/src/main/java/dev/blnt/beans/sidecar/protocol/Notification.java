package dev.blnt.beans.sidecar.protocol;

/**
 * An id-less outbound message the client renders but never awaits:
 * {@code progress} and {@code log} today.
 */
public record Notification(String method, Object params) {

    /** Payload of a {@code progress} notification. */
    public record Progress(String text) {}

    /** Payload of a {@code log} notification. */
    public record Log(String level, String logger, String text) {}
}
