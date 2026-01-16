use std::{collections::HashMap, ffi::OsStr, fmt::Display, path::Path};

use pinnacle_api::process::{Child, Command};

/// `Command` wrapper that spawns via `uwsm app`. this ensures processes are started within an
/// appropriate systemd slice, with a matching unit.
pub struct UwsmCommand {
    command: String,
    args: Vec<String>,
    once: bool,
    envs: HashMap<String, String>,
    unique: bool,
    pipe_stdin: bool,
    pipe_stdout: bool,
    pipe_stderr: bool,
    unit_type: Option<UnitType>,
    slice_selector: Option<SliceSelector>,
    unit_properties: Option<HashMap<String, String>>,
}

pub enum UnitType {
    Scope,
    Service,
}

impl Display for UnitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ut = match self {
            UnitType::Scope => "scope",
            UnitType::Service => "service",
        };
        write!(f, "{ut}")
    }
}

pub enum SliceSelector {
    App,
    Background,
    Session,
    Custom(String),
}

impl Display for SliceSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SliceSelector::App => "a",
            SliceSelector::Background => "b",
            SliceSelector::Session => "s",
            SliceSelector::Custom(name) => &name,
        };

        write!(f, "{s}")
    }
}

impl UwsmCommand {
    /// spawn a new `Command` wrapped by UWSM. the latter ensures the app is started within a systemd
    /// slice with the appropriate scope or session, under the compositor.
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
            unit_type: None,
            slice_selector: None,
            unit_properties: None,
        }
    }

    /// Adds multiple arguments to the command.
    pub fn args(self, args: impl IntoIterator<Item = impl ToString>) -> Self {
        UwsmCommand {
            args: args.into_iter().map(|s| ToString::to_string(&s)).collect(),
            ..self
        }
    }

    /// Causes this command to spawn the program exactly once in the compositor's lifespan.
    pub fn once(self) -> Self {
        UwsmCommand { once: true, ..self }
    }

    /// Spawns this command, returning the spawned process's standard io, if any.
    pub fn spawn(self) -> Option<Child> {
        Command::from(self).spawn()
    }

    /// Adds an argument to the command.
    pub fn arg(mut self, arg: impl ToString) -> Self {
        self.args.push(arg.to_string());
        self
    }

    /// Sets an environment variable that the process will spawn with.
    pub fn env(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.envs.insert(key.to_string(), value.to_string());
        self
    }

    /// Sets multiple environment variables that the process will spawn with.
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

    /// Causes this command to only spawn the program if it is the only instance currently running.
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Sets up a pipe to allow the config to write to the process's stdin.
    ///
    /// The pipe will be available through the spawned child's [`stdin`][Child::stdin].
    pub fn pipe_stdin(mut self) -> Self {
        self.pipe_stdin = true;
        self
    }

    /// Sets up a pipe to allow the config to read from the process's stdout.
    ///
    /// The pipe will be available through the spawned child's [`stdout`][Child::stdout].
    pub fn pipe_stdout(mut self) -> Self {
        self.pipe_stdout = true;
        self
    }

    /// Sets up a pipe to allow the config to read from the process's stderr.
    ///
    /// The pipe will be available through the spawned child's [`stderr`][Child::stderr].
    pub fn pipe_stderr(mut self) -> Self {
        self.pipe_stderr = true;
        self
    }

    /// Set the Systemd Unit type for the process -- this can either be `UnitType::Scope` or `UnitType::Service`
    pub fn unit_type(mut self, u: UnitType) -> Self {
        self.unit_type = Some(u);
        self
    }

    /// Choose the slice in which to run the process
    pub fn slice_selector(mut self, s: SliceSelector) -> Self {
        self.slice_selector = Some(s);
        self
    }

    /// Add a Systemd Unit property -- e.g. `PartOf` or `Description`
    ///
    /// this is useful for setting resource quotas.
    pub fn unit_property(mut self, key: impl ToString, value: impl ToString) -> Self {
        let mut up = if let Some(up) = self.unit_properties {
            up
        } else {
            HashMap::new()
        };
        up.insert(key.to_string(), value.to_string());
        self.unit_properties = Some(up);
        self
    }

    /// Add Systemd Unit properties -- e.g. `PartOf` or `Description` -- in bulk
    ///
    /// this is useful for setting resource quotas.
    pub fn unit_properties<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: ToString,
        V: ToString,
    {
        let mut up = if let Some(up) = self.unit_properties {
            up
        } else {
            HashMap::new()
        };
        up.extend(
            vars.into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self.unit_properties = Some(up);
        self
    }
}

impl From<UwsmCommand> for Command {
    fn from(value: UwsmCommand) -> Self {
        let app_name: &str = Path::new(&value.command)
            .file_prefix()
            .and_then(OsStr::to_str)
            .unwrap_or(&value.command);
        let mut uwsm_cmd = vec![
            "uwsm".to_string(),
            "app".to_string(),
            "-a".to_string(),
            app_name.to_string(),
        ];
        if let Some(ut) = value.unit_type {
            uwsm_cmd.append(&mut vec!["-t".to_string(), ut.to_string()]);
        }
        if let Some(s) = value.slice_selector {
            uwsm_cmd.append(&mut vec!["-s".to_string(), s.to_string()]);
        }
        for (k, v) in value.unit_properties.into_iter().flatten() {
            uwsm_cmd.append(&mut vec!["-p".to_string(), format!("{k}={v}")]);
        }
        let mut cmd = Command::with_shell(uwsm_cmd, &value.command);
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
