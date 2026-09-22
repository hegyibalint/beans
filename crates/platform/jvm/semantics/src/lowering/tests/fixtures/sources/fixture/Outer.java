package fixture;

public class Outer {
    protected static class ProtectedMember {}
    private class PrivateMember {}
    class PackageMember {}
    public interface MemberInterface {}
    public @interface MemberAnnotation {}
    public enum MemberEnum {}
    public record MemberRecord(int value) {}

    void localClass() {
        class Local {}
        new Local();
    }

    Object anonymousClass = new Object() {};
}
