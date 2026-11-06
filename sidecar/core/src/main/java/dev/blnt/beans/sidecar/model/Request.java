package dev.blnt.beans.sidecar.model;

import com.fasterxml.jackson.databind.JsonNode;

public record Request(JsonNode id, String method, JsonNode params) {}
