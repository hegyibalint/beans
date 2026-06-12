//! List every JVM installation on this machine:
//! `cargo run -p beans-toolchains --example list`

fn main() {
    let start = std::time::Instant::now();
    let installs = beans_toolchains::detect();
    let elapsed = start.elapsed();

    println!("{} installation(s) in {elapsed:.2?}\n", installs.len());
    for inst in &installs {
        let meta = inst
            .metadata
            .as_ref()
            .map(|m| {
                format!(
                    "{:<10} {:<24}",
                    m.version,
                    m.vendor.as_deref().unwrap_or("?")
                )
            })
            .unwrap_or_else(|| format!("{:<10} {:<24}", "?", "(no release file)"));
        println!(
            "  {} {} {}  [{}]",
            if inst.is_jdk() { "JDK" } else { "JRE" },
            meta,
            inst.java_home.display(),
            inst.sources.join(", ")
        );
    }
}
