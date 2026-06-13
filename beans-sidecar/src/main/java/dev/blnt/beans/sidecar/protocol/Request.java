package dev.blnt.beans.sidecar.protocol;

import com.fasterxml.jackson.databind.JsonNode;

/**
 * One inbound JSON-Lines message: {@code {id, method, params}}.
 * {@code params} stays a tree here; each duty binds it to its exact
 * params record via {@link StdioTransport#bind}.
 */
public record Request(Integer id, String method, JsonNode params) {}
