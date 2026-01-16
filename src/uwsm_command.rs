use std::collections::HashMap;

use pinnacle_api::process::{Child, Command};

pub struct UwsmCommand {
    command: String,
    args: Vec<String>,
    once: bool,
    envs: HashMap<String, String>,
    unique: bool,
    pipe_stdin: bool,
    pipe_stdout: bool,
    pipe_stderr: bool,
}

impl UwsmCommand {
    pub fn new(command: impl ToString) -> UwsmCommand {
        UwsmCommand {
            command: command.to_string(),
            args: Vec::new(),
            once: false,
            envs: HashMap::new(),
            unique: false,
            pipe_stdin: false,
            pipe_stdout: false,
            pipe_stderr: false,
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
        Command::from(self).spawn()
    }

    pub fn arg(mut self, arg: impl ToString) -> Self {
        self.args.push(arg.to_string());
        self
    }

    pub fn env(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.envs.insert(key.to_string(), value.to_string());
        self
    }

    pub fn envs<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: ToString,
        V: ToString,
    {
        self.envs.extend(
            vars.into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self
    }

    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    pub fn pipe_stdin(mut self) -> Self {
        self.pipe_stdin = true;
        self
    }

    pub fn pipe_stdout(mut self) -> Self {
        self.pipe_stdout = true;
        self
    }

    pub fn pipe_stderr(mut self) -> Self {
        self.pipe_stderr = true;
        self
    }
}

impl From<UwsmCommand> for Command {
    fn from(value: UwsmCommand) -> Self {
        let mut cmd = Command::with_shell(["uwsm", "app", "-a", &value.command], &value.command);
        cmd.args(value.args);
        cmd.envs(value.envs);
        if value.once {
            cmd.once();
        };
        if value.unique {
            cmd.unique();
        }
        if value.pipe_stdin {
            cmd.pipe_stdin();
        }
        if value.pipe_stdout {
            cmd.pipe_stdout();
        }
        if value.pipe_stderr {
            cmd.pipe_stderr();
        }
        cmd
    }
}
