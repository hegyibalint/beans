package dev.blnt.beans.sidecar.fixtures;

import dev.blnt.beans.sidecar.Handler;
import dev.blnt.beans.sidecar.Module;

public final class StdoutScreamer implements Module {

    @Override
    public void register(Handler handler) {
        System.out.println("stdout-screamer loaded");
        System.out.println("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"not-a-pong\"}");
    }
}
