package dev.blnt.beans.sidecar.protocol;

import com.fasterxml.jackson.databind.JsonNode;

/** One inbound message. {@code params} stays a tree until a handler binds it. */
public record Request(Integer id, String method, JsonNode params) {}
