use std::{
    fmt::Display,
    io::{self, Write},
};

use log::Record;

pub fn init() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .target(env_logger::Target::Stderr)
        .format(|buf, record| {
            let timestamp = buf.timestamp_millis();
            write_record(buf, timestamp, record)
        })
        .init();
}

fn write_record(
    writer: &mut impl Write,
    timestamp: impl Display,
    record: &Record<'_>,
) -> io::Result<()> {
    // Targets can be overridden; module_path!() starts with the emitting crate.
    // https://doc.rust-lang.org/std/macro.module_path.html
    let crate_name = record
        .module_path()
        .unwrap_or(record.target())
        .split("::")
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("unknown")
        .replace('_', "-");
    writeln!(writer, "{timestamp} [{crate_name}] {}", record.args())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(module_path: Option<&str>, target: &str) -> String {
        let record = Record::builder()
            .args(format_args!("Indexed project using TOML"))
            .module_path(module_path)
            .target(target)
            .level(log::Level::Info)
            .build();
        let mut output = Vec::new();
        write_record(&mut output, "2026-01-01T12:00:00.123Z", &record).unwrap();
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn nested_modules_use_the_crate_name() {
        assert_eq!(
            line(Some("beans_workspace_toml::parser::descriptor"), "unused"),
            "2026-01-01T12:00:00.123Z [beans-workspace-toml] Indexed project using TOML\n"
        );
    }

    #[test]
    fn crate_root_records_keep_the_crate_name() {
        assert_eq!(
            line(Some("beans_lsp"), "unused"),
            "2026-01-01T12:00:00.123Z [beans-lsp] Indexed project using TOML\n"
        );
    }

    #[test]
    fn custom_targets_do_not_change_the_crate_name() {
        assert_eq!(
            line(Some("beans_workspace_toml::parser"), "project_import"),
            "2026-01-01T12:00:00.123Z [beans-workspace-toml] Indexed project using TOML\n"
        );
    }

    #[test]
    fn records_without_a_module_path_fall_back_to_the_target() {
        assert_eq!(
            line(None, "external_crate::module"),
            "2026-01-01T12:00:00.123Z [external-crate] Indexed project using TOML\n"
        );
    }

    #[test]
    fn records_without_origin_use_an_unknown_label() {
        assert_eq!(
            line(None, ""),
            "2026-01-01T12:00:00.123Z [unknown] Indexed project using TOML\n"
        );
    }
}
