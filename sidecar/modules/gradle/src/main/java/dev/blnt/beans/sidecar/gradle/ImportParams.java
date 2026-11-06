package dev.blnt.beans.sidecar.gradle;

import java.util.Map;

public record ImportParams(String workspaceRoot, Map<String, String> options) {

    public String option(String key) {
        return options == null ? null : options.get(key);
    }
}
