package dev.blnt.beans.gradle.model;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import org.gradle.api.Project;
import org.gradle.api.artifacts.Configuration;
import org.gradle.api.artifacts.ConfigurationVariant;
import org.gradle.api.artifacts.PublishArtifact;
import org.gradle.api.plugins.jvm.internal.JvmFeatureInternal;
import org.gradle.api.provider.Provider;
import org.gradle.jvm.component.internal.JvmSoftwareComponentInternal;

final class JvmFeatures {
    static List<JvmFeatureModel> snapshot(Project project) {
        List<JvmFeatureModel> features = new ArrayList<>();
        for (JvmSoftwareComponentInternal component
                : project.getComponents().withType(JvmSoftwareComponentInternal.class)) {
            for (JvmFeatureInternal feature : component.getFeatures()) {
                List<String> artifacts = new ArrayList<>();
                collectArtifacts(configuration(feature, "getApiElementsConfiguration"), artifacts);
                collectArtifacts(configuration(feature, "getRuntimeElementsConfiguration"), artifacts);
                Collections.sort(artifacts);
                features.add(new Snapshots.Feature(feature.getName(), feature.getSourceSet().getName(),
                        artifacts));
            }
        }
        return features;
    }

    private static void collectArtifacts(Configuration configuration, List<String> paths) {
        addArtifacts(configuration.getOutgoing().getArtifacts(), paths);
        for (ConfigurationVariant variant : configuration.getOutgoing().getVariants()) {
            addArtifacts(variant.getArtifacts(), paths);
        }
    }

    private static void addArtifacts(Iterable<PublishArtifact> artifacts, List<String> paths) {
        for (PublishArtifact artifact : artifacts) {
            String path = artifact.getFile().toPath().toAbsolutePath().normalize().toString();
            if (!paths.contains(path)) {
                paths.add(path);
            }
        }
    }

    private static Configuration configuration(JvmFeatureInternal feature, String method) {
        // These internal getters changed from Configuration to providers; avoid return-type linkage.
        try {
            Object value = JvmFeatureInternal.class.getMethod(method).invoke(feature);
            if (value instanceof Provider<?>) {
                value = ((Provider<?>) value).get();
            }
            return (Configuration) value;
        } catch (ReflectiveOperationException error) {
            throw new IllegalStateException("Cannot read " + method + " for JVM feature "
                    + feature.getName(), error);
        }
    }

    private JvmFeatures() {}
}
