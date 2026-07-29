// The same theft, split across two files. See Burglar.java.
//
// Nothing in §6.6.1 mentions files, so the answer must not change.
class Safe {
    private int combination;

    int open() {
        return combination;
    }
}
