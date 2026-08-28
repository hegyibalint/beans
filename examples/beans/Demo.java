/**
 * A guided tour of what Beans answers today.
 *
 * Seven steps. Each says what to do and what you should see; the last one says
 * what deliberately does not work yet, so nothing here looks broken when it is
 * merely unbuilt.
 *
 * The unnamed package on purpose: this unit depends on nothing, so everything
 * below is decided inside this one directory. The scoping story — which unit
 * may see which — is the other example, in `a/` and `b/`.
 *
 * One setting changes what you see. `beans.toml` has `jdk_home` commented out,
 * so no runtime image is in the lake and `java.lang` names resolve to nothing.
 * Step 6 is the one that depends on it; `mise where java` prints the path.
 */
public class Demo {

    int count;
    Widget[] widgets;
    Widget grid[][];

    // ---------------------------------------------------------------- 1 ----
    /**
     * COMPLETION: what a caret can reach.
     *
     * Put the caret on the empty line and invoke completion (ctrl-space).
     *
     * Expect all three of JLS 6.1's namespaces at once, because an unqualified
     * caret is 6.5.2's *AmbiguousName* and the spec itself does not choose:
     *
     *   types      Demo, Widget, Neighbour, Base, Marker
     *   variables  total, factor, count, widgets, grid
     *   methods    describe(int), render(Widget[], Widget[]), shadowing(),
     *              navigation(Neighbour), diagnostics(Neighbour), notYet()
     *
     * A method row reads with its parameter types and inserts the bare name.
     * Types and not names, because JVMS 4.7.24 makes `MethodParameters`
     * optional and most compiled methods carry no parameter name to show.
     * The return type is the grey text to the right of the row.
     *
     * Note what is NOT there: `hidden` from Neighbour, because 6.6.1 keeps a
     * private field inside its own top level class.
     */
    void describe(int factor) {
        int total = count * factor;

    }

    // ---------------------------------------------------------------- 2 ----
    /**
     * COMPLETION: the type as it was written.
     *
     * Type `wid` and look at the detail beside the row, then `gri`.
     *
     *   widgets   Widget[]
     *   grid      Widget[][]
     *
     * Both used to read `Widget`. JLS 10.2 builds one type out of the brackets
     * on the type AND the brackets after the identifier, which is why `grid`
     * above is a `Widget[][]` though only one pair sits next to the name.
     *
     * The detail is the type as *written*, not as resolved — it is what an
     * editor shows and it costs no lookup.
     */
    void render(Widget[] items, Widget... more) {
        // Completion here offers `items` as `Widget[]` and `more` as `Widget[]`
        // too: 8.4.1 makes a variable arity parameter an array type, and 10.2
        // treats the ellipsis as a bracket pair. So `render(Widget[])` and
        // `render(Widget...)` declare the same parameter type.
        //
        // Complete `render` itself and the row reads
        //   render(Widget[], Widget[])          void
        // and accepting it writes `render`, not the parameters.
    }

    // ---------------------------------------------------------------- 3 ----
    /**
     * SHADOWING: the nearer declaration takes the name.
     *
     * Complete `cou` on the empty line. Exactly one row, and it is the local —
     * JLS 6.4.1 gives the spelling to the innermost declaration outright, so
     * the field is not offered as a second choice.
     *
     * Move the caret ABOVE the declaration and complete again: no row at all.
     * 6.3 starts a local's scope at its own declarator.
     */
    void shadowing() {
        int count = 1;

    }

    // ---------------------------------------------------------------- 4 ----
    /**
     * GO TO DECLARATION: press F12 on each marked name.
     *
     *   Widget      the class below
     *   Neighbour   another file in this unit — a file you never opened
     *   count       the field at the top
     *   describe    the method above
     *   this        the class Demo
     */
    void navigation(Neighbour neighbour) {
        Widget widget = null;
        this.count = 1;
        describe(2);
        int borrowed = neighbour.shared;
    }

    // ---------------------------------------------------------------- 5 ----
    /**
     * SUPERTYPES: F12 on a name in an `extends` or `implements` clause.
     *
     * Look at `Widget` below. F12 on `Base` lands on the class, F12 on
     * `Marker` lands on the interface. Each name in the list is its own
     * reference, so the caret has to land on the one it is inside.
     *
     * What does NOT work yet: Widget does not offer Base's members. See step 7.
     */

    // ---------------------------------------------------------------- 6 ----
    /**
     * DIAGNOSTICS: two different questions, two different squiggles.
     *
     * `hidden` is private to Neighbour, so this line carries
     * `inaccessible-member` — the name resolves, and 6.6.1 says this place may
     * not touch it. Resolution stays permissive on purpose: F12 still lands on
     * the declaration, because navigating to the thing you may not touch beats
     * pretending it is missing. It is the only squiggle in this file.
     *
     * The `String` below carries nothing, which is worth knowing rather than
     * guessing at. With `jdk_home` commented out no runtime image is in the
     * lake, so the name resolves to *nothing* — and an unknown type has no
     * diagnostic yet, so it stays silent. Uncomment `jdk_home` and it simply
     * resolves.
     *
     * The squiggle that does not appear here is `type-outside-scope`, and it
     * needs a third state: a type that exists somewhere in the workspace but
     * not in this unit's world. That is what `a/` and `b/` are for.
     */
    void diagnostics(Neighbour neighbour) {
        int blocked = neighbour.hidden;
        String silentWithoutAJdk = null;
    }

    // ---------------------------------------------------------------- 7 ----
    /**
     * NOT YET. Each of these is unbuilt rather than broken, and each is an
     * entry in TODO.md.
     *
     *   widget.<caret>        A qualified caret is offered nothing. The
     *                         receiver is known, the members are not walked.
     *
     *   toString()            No class offers it. Inherited members need the
     *                         hierarchy walk, and reaching java.lang.Object
     *                         needs the members of a compiled type.
     *
     *   ConcurrentHashMap     On the classpath, not in scope, and without
     *                         auto-import a row for it would not compile.
     *
     *   () -> {}              A lambda body reaches the model not at all, so
     *                         its parameters are invisible.
     *
     *   new Widget() { }      An anonymous class body likewise, and `this`
     *                         inside one wrongly answers the enclosing class.
     */
    void notYet() {
    }
}

class Base {
    int inherited;
}

interface Marker {
}

/** Step 5 is about this line. F12 on `Base`, then on `Marker`. */
class Widget extends Base implements Marker {
    int own;
}
