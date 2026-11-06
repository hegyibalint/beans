package dev.blnt.beans.sidecar;

import java.io.FileDescriptor;
import java.io.FileOutputStream;
import java.io.IOException;
import java.util.ServiceLoader;

public final class Main {

    public static void main(String[] args) throws IOException {
        var input = System.in;
        var output = new FileOutputStream(FileDescriptor.out);
        System.setOut(System.err);

        var handler = new Handler(new Rpc(input, output));
        for (var module : ServiceLoader.load(Module.class)) {
            module.register(handler);
        }
        handler.run();
    }
}
