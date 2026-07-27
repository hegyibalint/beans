package dev.blnt.beans.sidecar.protocol;

/** An id-less outbound message the client renders but never awaits. */
public record Notification(String method, Object params) {

    public record Progress(String text) {}

    public record Log(String level, String logger, String text) {}
}
