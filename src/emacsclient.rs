use std::{collections::BTreeMap, fmt::Display};

use itertools::Itertools;
use pinnacle_api::process::{Child, Command};
use tokio::io::{AsyncRead, AsyncReadExt};
use users::get_current_uid;

use crate::uwsm_command::UwsmCommand;

#[derive(Debug, Clone, Copy, Default)]
pub enum ClientType {
    #[default]
    Graphical,
    Terminal,
}

#[derive(Debug, Clone, Default)]
pub struct EmacsClient {
    eval: Option<String>,
    frame_parameters: Option<BTreeMap<String, String>>,
    socket: Option<String>,
    graphical: Option<ClientType>,
}

pub struct EmacsInfo {
    pub output: Option<String>,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
}

async fn read_fd<T>(fd: Option<&mut T>) -> Option<String>
where
    T: AsyncRead + Unpin,
{
    if let Some(handle) = fd
        && let mut o = String::new()
        && let Ok(_) = handle.read_to_string(&mut o).await
    {
        Some(o)
    } else {
        None
    }
}

impl EmacsClient {
    /// create an empty parameter builder for `emacsclient`
    pub fn new() -> Self {
        Default::default()
    }

    /// pass an elisp expression to evaluate:
    ///
    /// ```rust
    ///   EmacsClient::new().eval("(daemonp)").build()
    /// ```
    ///
    /// produces the equivalent of:
    /// ```sh
    ///   emacsclient -e '(daemonp)'
    /// ```
    pub fn eval(mut self, command: impl Display) -> Self {
        self.eval = Some(command.to_string());
        self
    }

    /// add a single parameter to the `-F` parameter list:
    ///
    /// ```rust
    ///   EmacsClient::new().frame_parameter("name", "\"emacsclient\"").build()
    /// ```
    ///
    /// produces the equivalent of:
    /// ```sh
    ///   emacsclient -F '((name . "emacsclient"))'
    /// ```
    pub fn frame_parameter(mut self, key: impl Display, value: impl Display) -> Self {
        let mut frame_parameters = self.frame_parameters.unwrap_or_default();
        frame_parameters.insert(key.to_string(), value.to_string());

        self.frame_parameters = Some(frame_parameters);
        self
    }

    /// emacsclient `-F` -- provide frame parameters to adjust how emacs presents the frame to the window manager.
    ///
    /// this version allows bulk insert. see [Self::frame_parameter]
    pub fn frame_parameters(
        mut self,
        fps: impl IntoIterator<Item = (impl Display, impl Display)>,
    ) -> Self {
        let mut frame_parameters = self.frame_parameters.unwrap_or_default();
        for (k, v) in fps.into_iter() {
            frame_parameters.insert(k.to_string(), v.to_string());
        }

        self.frame_parameters = Some(frame_parameters);
        self
    }

    /// ensure the user daemon socket is provided in the command args
    ///
    /// i.e. `-s /run/user/$UID/emacs/server`
    pub fn attach_user_socket(mut self) -> Self {
        let uid = get_current_uid();
        self.socket = Some(format!("/run/user/{uid}/emacs/server"));
        self
    }

    /// spawn a graphical frame.
    ///
    /// emacsclient `-c` -- spawn a graphical frame
    pub fn graphical_frame(mut self) -> Self {
        self.graphical = Some(ClientType::Graphical);
        self
    }

    /// spawn a terminal frame.
    ///
    /// emacsclient `-t` -- hard to see how this is useful from the desktop but it's here for completeness
    pub fn terminal_frame(mut self) -> Self {
        self.graphical = Some(ClientType::Terminal);
        self
    }

    /// construct the emacsclient argument list
    pub fn build(&self) -> Vec<String> {
        let mut args = BTreeMap::<String, String>::new();

        if let Some(eval) = &self.eval {
            args.insert("-e".to_owned(), eval.clone());
        }

        if let Some(frame_parameters) = &self.frame_parameters {
            let fps = frame_parameters
                .iter()
                .map(|(k, v)| format!("({k} . {v})"))
                .join(" ");
            args.insert("-F".to_owned(), format!("({fps})"));
        }

        if let Some(socket) = &self.socket {
            args.insert("-s".to_owned(), socket.clone());
        }

        if let Some(graphical) = self.graphical.map(|g| match g {
            ClientType::Graphical => "-c".to_owned(),
            ClientType::Terminal => "-t".to_owned(),
        }) {
            args.insert(graphical, "".to_owned());
        }

        args.into_iter()
            .flat_map(|(k, v)| [k, v])
            .filter(|a| !a.is_empty())
            .collect()
    }

    /// spawn emacsclient as an external command, using a systemd slice when appropriate.
    ///
    /// captures stdout and stderr when no frame is requested (i.e. eliding `-c` and `-t`)
    pub fn spawn(&self) -> Option<Child> {
        match &self.graphical {
            Some(ClientType::Graphical) | Some(ClientType::Terminal) => {
                UwsmCommand::new("emacsclient").args(self.build()).spawn()
            }
            None => Command::new("emacsclient")
                .args(self.build())
                .pipe_stdout()
                .pipe_stderr()
                .spawn(),
        }
    }

    /// run the provided emacsclient command to completion, collecting stdout, stderr, and the exit code.
    ///
    /// this is primarily useful for eval commands that immediately exit.
    pub async fn run(&self) -> Option<EmacsInfo> {
        if let Some(mut child) = self.spawn()
            && let output = read_fd(child.stdout.as_mut()).await
            && let error = read_fd(child.stderr.as_mut()).await
            && let res = child.wait_async().await
        {
            Some(EmacsInfo {
                output,
                error,
                exit_code: res.exit_code,
            })
        } else {
            None
        }
    }
}
