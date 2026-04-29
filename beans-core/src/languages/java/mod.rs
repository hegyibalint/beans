//! Java language module.
//!
//! Per ADR-0004 / ADR-0019 each JVM language has its own module under
//! [`crate::languages`]. Java has no kinds beyond what the JVM projection
//! ([`crate::jvm`]) already covers — classes, interfaces, enums, records,
//! annotations, methods, fields, and so on are all JVM-shaped — so this
//! module is a placeholder for now and will receive the Java parser, type
//! resolution, and rule set in subsequent migration steps.
