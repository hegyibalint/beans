package dev.blnt.beans.sidecar.logging;

import dev.blnt.beans.sidecar.protocol.StdioTransport;
import org.slf4j.Marker;
import org.slf4j.event.Level;
import org.slf4j.helpers.LegacyAbstractLogger;
import org.slf4j.helpers.MessageFormatter;

/**
 * SLF4J logger that emits {@code {"method":"log"}} notifications on the
 * protocol channel — the Tooling API's internal logging becomes part of
 * the JSON-Lines stream instead of being discarded (or worse, polluting
 * stdout as raw text). INFO and above; debug/trace stay off unless we
 * ever grow a verbosity handshake.
 */
final class RpcLogger extends LegacyAbstractLogger {

    RpcLogger(String name) {
        this.name = name;
    }

    @Override
    public boolean isTraceEnabled() {
        return false;
    }

    @Override
    public boolean isDebugEnabled() {
        return false;
    }

    @Override
    public boolean isInfoEnabled() {
        return true;
    }

    @Override
    public boolean isWarnEnabled() {
        return true;
    }

    @Override
    public boolean isErrorEnabled() {
        return true;
    }

    @Override
    protected String getFullyQualifiedCallerName() {
        return null;
    }

    @Override
    protected void handleNormalizedLoggingCall(
            Level level, Marker marker, String pattern, Object[] args, Throwable throwable) {
        String text = MessageFormatter.basicArrayFormat(pattern, args);
        if (throwable != null) {
            text = text + " (" + throwable.getClass().getSimpleName() + ": "
                    + throwable.getMessage() + ")";
        }
        StdioTransport.INSTANCE.log(level.toString().toLowerCase(), name, text);
    }
}
