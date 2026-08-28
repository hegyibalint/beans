/**
 * A second file in the demo unit, so steps 4 and 6 cross a file boundary.
 *
 * Nothing here is meant to be read. It exists so that F12 from
 * `Presentation.navigation` lands in a file you never opened, and so that
 * `hidden` can be out of reach from a class that is not this one.
 */
public class Neighbour {

    /** Reachable from Presentation: same package, no modifier. */
    int shared;

    /** JLS 6.6.1 keeps this inside Neighbour's own top level class. */
    private int hidden;

    int usesItsOwn() {
        return hidden;
    }
}
