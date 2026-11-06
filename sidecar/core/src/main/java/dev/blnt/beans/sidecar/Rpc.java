package dev.blnt.beans.sidecar;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.blnt.beans.sidecar.model.Response;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;

public final class Rpc {

    private final BufferedReader input;
    private final BufferedWriter output;
    private final ObjectMapper mapper = new ObjectMapper()
            .enable(DeserializationFeature.FAIL_ON_TRAILING_TOKENS);

    public Rpc(InputStream input, OutputStream output) {
        this.input = new BufferedReader(new InputStreamReader(input, StandardCharsets.UTF_8));
        this.output = new BufferedWriter(new OutputStreamWriter(output, StandardCharsets.UTF_8));
    }

    public JsonNode receive() throws IOException {
        String line;
        while ((line = input.readLine()) != null) {
            if (!line.isBlank()) {
                return mapper.readTree(line);
            }
        }
        return null;
    }

    public synchronized void send(Response response) throws IOException {
        output.write(mapper.writeValueAsString(response));
        output.write('\n');
        output.flush();
    }
}
