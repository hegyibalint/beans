package dev.blnt.beans.sidecar.model;

import com.fasterxml.jackson.annotation.JsonSubTypes;
import com.fasterxml.jackson.annotation.JsonTypeInfo;

/** Matches Rust's externally tagged enum representation. */
@JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.WRAPPER_OBJECT)
@JsonSubTypes(@JsonSubTypes.Type(value = ResourceFilter.File.class, name = "File"))
public sealed interface ResourceFilter {

    record File(String root) implements ResourceFilter {}
}
