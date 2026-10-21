use std::collections::BTreeSet;

use beans_core::event::Command;
use crossbeam_channel::Sender;

use super::TomlWorkspace;

impl TomlWorkspace {
    pub fn start(&self, sender: &Sender<Command>) -> Result<(), String> {
        let homes: BTreeSet<_> = self
            .model()
            .units
            .values()
            .filter_map(|unit| unit.jdk_home.as_ref())
            .collect();
        for home in homes {
            sender
                .send(Command::ProcessJdkHome(home.clone()))
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}
