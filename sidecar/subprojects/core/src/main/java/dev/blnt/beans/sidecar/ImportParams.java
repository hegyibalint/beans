package dev.blnt.beans.sidecar;

import java.util.Map;

/**
 * Params of {@code build/import}. {@code options} carries per-tool launch knobs
 * (an installation to use instead of a wrapper, say) and never carries model
 * data — the model is typed or it is not in the protocol.
 */
public record ImportParams(String workspaceRoot, Map<String, String> options) {

    public String option(String key) {
        return options == null ? null : options.get(key);
    }
}
