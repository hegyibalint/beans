package dev.blnt.beans.sidecar.logging;

import org.slf4j.ILoggerFactory;
import org.slf4j.IMarkerFactory;
import org.slf4j.helpers.BasicMDCAdapter;
import org.slf4j.helpers.BasicMarkerFactory;
import org.slf4j.spi.MDCAdapter;
import org.slf4j.spi.SLF4JServiceProvider;

import java.util.concurrent.ConcurrentHashMap;

/**
 * The process-wide SLF4J binding. Without it a library's logging either
 * disappears or reaches a console we do not own.
 */
public final class RpcLoggerServiceProvider implements SLF4JServiceProvider {

    private final ConcurrentHashMap<String, RpcLogger> loggers = new ConcurrentHashMap<>();
    private final IMarkerFactory markerFactory = new BasicMarkerFactory();
    private final MDCAdapter mdcAdapter = new BasicMDCAdapter();

    @Override
    public ILoggerFactory getLoggerFactory() {
        return name -> loggers.computeIfAbsent(name, RpcLogger::new);
    }

    @Override
    public IMarkerFactory getMarkerFactory() {
        return markerFactory;
    }

    @Override
    public MDCAdapter getMDCAdapter() {
        return mdcAdapter;
    }

    @Override
    public String getRequestedApiVersion() {
        return "2.0.99";
    }

    @Override
    public void initialize() {}
}
