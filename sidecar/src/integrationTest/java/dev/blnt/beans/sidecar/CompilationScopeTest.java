package dev.blnt.beans.sidecar;

import com.fasterxml.jackson.databind.ObjectMapper;
import dev.blnt.beans.sidecar.model.CompilationScope;
import dev.blnt.beans.sidecar.model.ResourceFilter;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;

class CompilationScopeTest {

    private final ObjectMapper mapper = new ObjectMapper();

    @Test
    void empty_scopes_serialize_as_an_array() throws Exception {
        assertEquals(mapper.readTree("[]"), mapper.valueToTree(List.<CompilationScope>of()));
    }

    @Test
    void unrestricted_and_empty_dependencies_are_distinct() throws Exception {
        var inputs = List.<ResourceFilter>of(new ResourceFilter.File("/consumer"));
        assertEquals(mapper.readTree("""
                [{"inputs":[{"File":{"root":"/consumer"}}],"dependencies":null}]
                """), mapper.valueToTree(List.of(new CompilationScope(inputs, null))));
        assertEquals(mapper.readTree("""
                [{"inputs":[{"File":{"root":"/consumer"}}],"dependencies":[]}]
                """), mapper.valueToTree(List.of(new CompilationScope(inputs, List.of()))));
    }

    @Test
    void scope_and_filter_order_match_rust_serde() throws Exception {
        // Build an import result with ordered filters and an empty scope.
        var scopes = List.of(
                new CompilationScope(
                        List.of(
                                new ResourceFilter.File("/project/src"),
                                new ResourceFilter.File("/project/generated")),
                        List.of(
                                new ResourceFilter.File("/libs/b.jar"),
                                new ResourceFilter.File("/libs/a.jar"))),
                new CompilationScope(List.of(), List.of()));

        // Check the shared JSON representation.
        var expected = mapper.readTree("""
                [
                    {
                        "inputs":[
                            {"File":{"root":"/project/src"}},
                            {"File":{"root":"/project/generated"}}
                        ],
                        "dependencies":[
                            {"File":{"root":"/libs/b.jar"}},
                            {"File":{"root":"/libs/a.jar"}}
                        ]
                    },
                    {"inputs":[],"dependencies":[]}
                ]
                """);
        assertEquals(expected, mapper.valueToTree(scopes));
    }
}
