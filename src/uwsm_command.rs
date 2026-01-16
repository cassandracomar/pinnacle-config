use pinnacle_api::process::{Child, Command};

pub struct UwsmCommand {
    command: String,
    args: Vec<String>,
    once: bool,
}

impl UwsmCommand {
    pub fn new(command: impl ToString) -> UwsmCommand {
        UwsmCommand {
            command: command.to_string(),
            args: Vec::new(),
            once: false,
        }
    }

    pub fn args(self, args: impl IntoIterator<Item = impl ToString>) -> Self {
        UwsmCommand {
            args: args.into_iter().map(|s| ToString::to_string(&s)).collect(),
            ..self
        }
    }

    pub fn once(self) -> Self {
        UwsmCommand { once: true, ..self }
    }

    pub fn spawn(self) -> Option<Child> {
        if self.once {
            Command::from(self).once().spawn()
        } else {
            Command::from(self).spawn()
        }
    }
}

impl From<UwsmCommand> for Command {
    fn from(value: UwsmCommand) -> Self {
        let mut cmd = Command::with_shell(["uwsm", "app", "-a", &value.command], &value.command);
        cmd.args(value.args);
        if value.once {
            cmd.once();
        };
        cmd
    }
}
