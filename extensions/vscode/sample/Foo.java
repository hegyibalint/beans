package org.beans.showcase;

import java.time.Instant;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Predicate;

@Foo.Traced("showcase")
public final class Foo<T extends Comparable<? super T>> implements Iterable<Foo.Event<T>> {
    private final Map<String, List<Event<T>>> timelines = new ConcurrentHashMap<>();
    private final Comparator<T> ordering;

    public Foo(Comparator<T> ordering) {
        this.ordering = ordering;
    }

    public Event<T> record(String stream, T value, Severity severity) {
        Event<T> event = new Event<>(stream, value, severity, Instant.now());
        timelines.computeIfAbsent(stream, ignored -> new ArrayList<>()).add(event);
        return event;
    }

    public List<Event<T>> query(String stream, Predicate<Event<T>> predicate) {
        class QueryPlan {
            List<Event<T>> execute() {
                return timelines.getOrDefault(stream, List.of())
                    .stream()
                    .filter(predicate)
                    .sorted((left, right) -> ordering.compare(left.value(), right.value()))
                    .toList();
            }
        }

        return new QueryPlan().execute();
    }

    @Override
    public Iterator<Event<T>> iterator() {
        return timelines.values().stream().flatMap(List::stream).iterator();
    }

    public record Event<V>(
        String stream,
        V value,
        Severity severity,
        Instant recordedAt
    ) implements Signal {
        @Override
        public String description() {
            return stream + ": " + value;
        }
    }

    public sealed interface Signal permits Event, Alert {
        String description();
    }

    public static final class Alert implements Signal {
        private final Severity severity;
        private final String message;

        public Alert(Severity severity, String message) {
            this.severity = severity;
            this.message = message;
        }

        @Override
        public String description() {
            return severity + ": " + message;
        }
    }

    public enum Severity {
        TRACE(0),
        INFO(1),
        WARNING(2),
        CRITICAL(3);

        private final int weight;

        Severity(int weight) {
            this.weight = weight;
        }

        public boolean atLeast(Severity other) {
            return weight >= other.weight;
        }
    }

    public @interface Traced {
        String value();
    }

    public static class Builder<U extends Comparable<? super U>> {
        private Comparator<U> ordering = Comparator.naturalOrder();

        public Builder<U> orderedBy(Comparator<U> comparator) {
            ordering = comparator;
            return this;
        }

        public Foo<U> build() {
            return new Foo<>(ordering);
        }
    }
}

interface SnapshotStore<K, V> {
    void save(K key, V value);

    V load(K key);
}

record InMemorySnapshot<K, V>(K key, V value, Instant createdAt) {}
