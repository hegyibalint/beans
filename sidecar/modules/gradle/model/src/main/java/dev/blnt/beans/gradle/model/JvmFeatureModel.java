package dev.blnt.beans.gradle.model;

import java.util.List;

/** A JVM feature's source-set association and declared outgoing artifact roots. */
public interface JvmFeatureModel {
    String getName();
    String getSourceSetName();
    /** API/runtime artifacts, including secondary variants; outputs need not exist. */
    List<String> getArtifacts();
}
