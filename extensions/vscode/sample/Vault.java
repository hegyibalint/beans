// Two top level classes in one file (JLS 26 §7.6).
//
// JLS 26 §6.6.1 permits private access when it "occurs from within the body of
// the top level class or interface that encloses the declaration". The unit is
// the whole outermost body, so the reach is wider than the declaring class and
// narrower than the file. Both reads below are spelled `v.secret`.
class Vault {
    private int secret;

    static class Inner {
        int peek(Vault v) {
            // Legal. Inner sits inside Vault's top level body.
            return v.secret;
        }
    }
}

class Thief {
    int steal(Vault v) {
        // Error. Thief is its own top level class, so it is outside.
        return v.secret;
    }
}
