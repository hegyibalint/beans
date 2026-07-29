// The two file half of Safe.java.
class Burglar {
    int crack(Safe s) {
        // Error, exactly as in Vault.java. A different file is just another way
        // of being a different top level class.
        return s.combination;
    }
}
