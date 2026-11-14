package dev.blnt.beans.sidecar.model;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

public record Module(
        List<Classpath> classpaths,
        @JsonProperty("artifact_sources") List<ArtifactSources> artifactSources) {}
