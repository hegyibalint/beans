//! Pick the best JVM for a requirement:
//! `cargo run -p beans-toolchains --example pick -- [min_major] [--jdk] [--exact]`

use beans_toolchains::ToolchainSpec;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let major: Option<u32> = args.iter().find_map(|a| a.parse().ok());
    let spec = ToolchainSpec {
        min_major: if args.iter().any(|a| a == "--exact") {
            None
        } else {
            major
        },
        exact_major: args
            .iter()
            .any(|a| a == "--exact")
            .then_some(major)
            .flatten(),
        require_jdk: args.iter().any(|a| a == "--jdk"),
    };

    let mut installs = beans_toolchains::detect();
    match beans_toolchains::select(&mut installs, &spec) {
        Some(inst) => {
            let meta = inst.metadata.as_ref().unwrap();
            println!(
                "{} {} ({}) -> {}",
                if inst.is_jdk() { "JDK" } else { "JRE" },
                meta.version,
                meta.vendor.as_deref().unwrap_or("?"),
                inst.java_home.display()
            );
        }
        None => {
            println!("no installation satisfies {spec:?}");
            std::process::exit(1);
        }
    }
}
