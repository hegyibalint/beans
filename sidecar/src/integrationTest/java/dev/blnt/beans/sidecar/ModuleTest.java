package dev.blnt.beans.sidecar;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.blnt.beans.sidecar.model.ArtifactSources;
import dev.blnt.beans.sidecar.model.Classpath;
import dev.blnt.beans.sidecar.model.Location;
import dev.blnt.beans.sidecar.model.Module;
import dev.blnt.beans.sidecar.model.ResourceFilter;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;

class ModuleTest {
    private final ObjectMapper mapper = new ObjectMapper();

    @Test
    void empty_modules_serialize_as_an_array() throws Exception {
        assertEquals(mapper.readTree("[]"), mapper.valueToTree(List.<Module>of()));
    }

    @Test
    void unrestricted_and_empty_classpaths_are_distinct() throws Exception {
        var inputs = List.<ResourceFilter>of(new ResourceFilter.File("/consumer"));
        for (var classpath : List.of(new Classpath(inputs, null), new Classpath(inputs, List.of()))) {
            var module = new Module(List.of(classpath), List.of());
            var json = mapper.valueToTree(module);
            assertEquals(mapper.valueToTree(classpath.entries()), json.at("/classpaths/0/entries"));
            assertEquals(module, mapper.treeToValue(json, Module.class));
        }
    }

    @Test
    void modules_and_artifact_associations_match_rust_serde() throws Exception {
        var modules = List.of(new Module(
                List.of(new Classpath(
                        List.of(new ResourceFilter.File("/project/app/App.java")),
                        List.of(new ResourceFilter.File("/project/lib/lib.jar")))),
                List.of(new ArtifactSources(
                        List.of(Location.file("/project/lib/classes"), Location.file("/project/lib/lib.jar")),
                        List.of(new ResourceFilter.File("/project/lib/Lib.java"))))));
        var expected = mapper.readTree("""
                [{
                    "classpaths":[{
                        "inputs":[{"File":{"root":"/project/app/App.java"}}],
                        "entries":[{"File":{"root":"/project/lib/lib.jar"}}]
                    }],
                    "artifact_sources":[{
                        "artifacts":[
                            [{"File":"/project/lib/classes"}],
                            [{"File":"/project/lib/lib.jar"}]
                        ],
                        "sources":[{"File":{"root":"/project/lib/Lib.java"}}]
                    }]
                }]
                """);
        assertEquals(expected, mapper.valueToTree(modules));
        assertEquals(modules, mapper.readValue(expected.toString(), new TypeReference<List<Module>>() {}));
    }

    @Test
    void locations_preserve_nested_and_virtual_entries() throws Exception {
        var locations = List.of(
                new Location(List.of(new Location.Entry.File("/project/app.zip"),
                        new Location.Entry.Container("lib/a.jar"),
                        new Location.Entry.Container("p/C.class"))),
                new Location(List.of(new Location.Entry.Virtual("UNTITLED:My%20Type.java"))));
        var expected = mapper.readTree("""
                [
                    [{"File":"/project/app.zip"},{"Container":"lib/a.jar"},{"Container":"p/C.class"}],
                    [{"Virtual":"UNTITLED:My%20Type.java"}]
                ]
                """);
        assertEquals(expected, mapper.valueToTree(locations));
        assertEquals(locations, mapper.readValue(expected.toString(), new TypeReference<List<Location>>() {}));
    }
}
