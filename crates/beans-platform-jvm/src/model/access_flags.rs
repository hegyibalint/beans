// cafebabe's typed flags do not implement Serde; persist their JVM bit masks.
macro_rules! access_flags {
    ($module:ident, $flags:ty) => {
        pub(super) mod $module {
            use serde::{Deserialize, Deserializer, Serializer};

            pub fn serialize<S: Serializer>(
                flags: &$flags,
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                serializer.serialize_u16(flags.bits())
            }

            pub fn deserialize<'de, D: Deserializer<'de>>(
                deserializer: D,
            ) -> Result<$flags, D::Error> {
                let bits = u16::deserialize(deserializer)?;
                <$flags>::from_bits(bits)
                    .ok_or_else(|| serde::de::Error::custom("invalid JVM access flags"))
            }
        }
    };
}

access_flags!(class, cafebabe::ClassAccessFlags);
access_flags!(field, cafebabe::FieldAccessFlags);
access_flags!(method, cafebabe::MethodAccessFlags);
