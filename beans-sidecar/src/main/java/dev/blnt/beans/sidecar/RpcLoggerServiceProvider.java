package dev.blnt.beans.sidecar;

import org.slf4j.ILoggerFactory;
import org.slf4j.IMarkerFactory;
import org.slf4j.helpers.BasicMDCAdapter;
import org.slf4j.helpers.BasicMarkerFactory;
import org.slf4j.spi.MDCAdapter;
import org.slf4j.spi.SLF4JServiceProvider;

import java.util.concurrent.ConcurrentHashMap;

/**
 * SLF4J binding routing all logging (the Tooling API's included) onto
 * the JSON-Lines protocol as {@code log} notifications. Registered via
 * {@code META-INF/services/org.slf4j.spi.SLF4JServiceProvider}.
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
