package demo.scopes;

import java.util.List;
import static java.util.Collections.emptyList;

public abstract class Outer<T extends Number & Comparable<T>>
        extends base.Parent<T>
        implements java.io.Serializable, Comparable<Outer<? extends T>> {
    protected static final class Nested {}

    interface Contract {}

    enum Choice {
        FIRST;

        class Detail {}
    }

    record Pair<A, B>(A left, B right) {}

    @interface Marker {}
}

final class Sibling {
    Outer<?> value;
}
