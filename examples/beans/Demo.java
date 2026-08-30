/**
 * A guided tour of what Beans answers today. Eight steps: what to type, what
 * you should see. Step 8 is what does not work yet.
 *
 * `beans.toml` has `jdk_home` commented out, so no JDK is in the lake and
 * `java.lang` names resolve to nothing. Only step 6 depends on that.
 */
public class Demo {

    int count;
    Widget[] widgets;
    Widget grid[][];

    // ---------------------------------------------------------------- 1 ----
    /**
     * Ctrl-space on the empty line. All three of JLS 6.1's namespaces at once,
     * because an unqualified caret is 6.5.2's *AmbiguousName*:
     *
     *   types      Demo, Widget, Neighbour, Base, Marker
     *   variables  total, factor, count, widgets, grid
     *   methods    describe(int factor), render(Widget[] items, Widget[] more),
     *              shadowing(), navigation(Neighbour neighbour),
     *              diagnostics(Neighbour neighbour), members(Widget widget),
     *              notYet()
     *
     * Not there: `hidden` from Neighbour. 6.6.1 keeps a private field at home.
     */
    void describe(int factor) {
        int total = count * factor;
        // ctrl-space on the next line

    }

    // ---------------------------------------------------------------- 2 ----
    /**
     * Type `wid`, then `gri`, and read the detail beside the row:
     *
     *   widgets   Widget[]
     *   grid      Widget[][]
     *
     * 10.2 builds one type from the brackets on the type AND the brackets after
     * the identifier, which is why `grid` is two-dimensional.
     */
    void render(Widget[] items, Widget... more) {
        // Both parameters read `Widget[]`: 8.4.1 makes a variable arity
        // parameter an array type. Completing `render` writes the bare name.
    }

    // ---------------------------------------------------------------- 3 ----
    /**
     * Complete `cou` on the empty line. One row, the local — 6.4.1 gives the
     * spelling to the innermost declaration outright.
     *
     * Move the caret above the declaration and retry: no row. 6.3 starts a
     * local's scope at its own declarator.
     */
    void shadowing() {
        int count = 1;
        // complete `cou` on the next line

    }

    // ---------------------------------------------------------------- 4 ----
    /**
     * F12 on each marked name.
     *
     *   Widget      the class below
     *   Neighbour   another file in this unit — one you never opened
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
     * F12 on `Base`, then on `Marker`, in Widget's declaration at the bottom.
     * Each name in the clause is its own reference.
     */

    // ---------------------------------------------------------------- 6 ----
    /**
     * `hidden` is private to Neighbour, so it carries `inaccessible-member` —
     * the only squiggle in this file. F12 still lands on it: navigating to what
     * you may not touch beats pretending it is missing.
     *
     * `String` carries nothing. With no JDK in the lake the name resolves to
     * nothing, and an unknown type has no diagnostic yet.
     */
    void diagnostics(Neighbour neighbour) {
        int blocked = neighbour.hidden;
        String silentWithoutAJdk = null;
    }

    // ---------------------------------------------------------------- 7 ----
    /**
     * Type `widget.` on the empty line. One row:
     *
     *   own    int
     *
     * `inherited` is Base's, and we do not walk the hierarchy yet. Step 8.
     *
     * The list is not the names in scope — 6.5.6.2 asks for a member, so
     * `count` after the dot would be wrong rather than unhelpful.
     */
    void members(Widget widget) {
        // type `widget.` on the next line

    }

    // ---------------------------------------------------------------- 8 ----
    /**
     * NOT YET. Unbuilt rather than broken; each is an entry in TODO.md.
     *
     *   widget.inherited      a supertype's members need the hierarchy walk
     *   toString()            that walk, plus the members of a compiled type
     *   this.<caret>          a trailing dot eats the rest of the class body,
     *                         so the list is short and carries stray types
     *   ConcurrentHashMap     on the classpath, not in scope, no auto-import
     *   () -> {}              a lambda body reaches the model not at all
     *   new Widget() { }      likewise, and `this` inside one answers wrongly
     */
    void notYet() {
    }
}

class Base {
    int inherited;
}

interface Marker {
}

/** Step 5 is about this line. */
class Widget extends Base implements Marker {
    int own;
}
