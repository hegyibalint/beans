//! Report every JVM installation on this machine, newest first:
//! `cargo run -p beans-toolchains --example list`

use beans_toolchains::JavaInstallation;

fn main() {
    let start = std::time::Instant::now();
    let mut installs = beans_toolchains::detect();
    let elapsed = start.elapsed();

    // Newest first; unprobed (no release file) entries sink to the end.
    installs.sort_by(|a, b| {
        let key = |i: &JavaInstallation| {
            i.metadata
                .as_ref()
                .map(|m| (m.major, m.version_segments()))
                .unwrap_or((0, Vec::new()))
        };
        key(b).cmp(&key(a)).then(a.java_home.cmp(&b.java_home))
    });

    let home = std::env::var("HOME").unwrap_or_default();
    let tilde = |p: &std::path::Path| {
        let s = p.display().to_string();
        match s.strip_prefix(&home) {
            Some(rest) if !home.is_empty() => format!("~{rest}"),
            _ => s,
        }
    };

    let rows: Vec<(String, String, String, String, String)> = installs
        .iter()
        .map(|i| {
            let (major, version, vendor) = match &i.metadata {
                Some(m) => (
                    m.major.to_string(),
                    m.version.clone(),
                    m.vendor.clone().unwrap_or_else(|| "unknown vendor".into()),
                ),
                None => ("?".into(), "?".into(), "no release file".into()),
            };
            let kind = if i.is_jdk() { "JDK" } else { "JRE" };
            (major, kind.to_string(), version, vendor, tilde(&i.java_home))
        })
        .collect();

    let w = |f: fn(&(String, String, String, String, String)) -> usize| {
        rows.iter().map(f).max().unwrap_or(0)
    };
    let (w0, w2, w3) = (w(|r| r.0.len()), w(|r| r.2.len()), w(|r| r.3.len()));

    println!(
        "Identified {} JVM installation(s) in {elapsed:.0?}:\n",
        rows.len()
    );
    for (major, kind, version, vendor, path) in &rows {
        println!("  Java {major:>w0$}  {kind}  {version:<w2$}  {vendor:<w3$}  {path}");
    }
}
