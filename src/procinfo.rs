use std::time::Duration;

use pinnacle_api::process::{Child, Command};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    time::sleep,
};

pub struct ProcInfo {
    pub output: Option<String>,
    pub error: Option<String>,
    pub exit_code: Option<i32>,
}

pub async fn read_fd<T>(fd: Option<&mut T>) -> Option<String>
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

pub async fn collect_proc_info(child: Option<Child>) -> Option<ProcInfo> {
    if let Some(mut child) = child
        && let output = read_fd(child.stdout.as_mut()).await
        && let error = read_fd(child.stderr.as_mut()).await
        && let res = child.wait_async().await
    {
        Some(ProcInfo {
            output,
            error,
            exit_code: res.exit_code,
        })
    } else {
        None
    }
}

/// we can test if a daemon is running with a trial command, with expected output.
/// the daemon is running if all of the following are true:
/// 1. the trial command gave a zero exit code
/// 2. the command produced no stderr output
/// 3. the command produced exactly the expected output on stdout (after trimming)
pub fn is_running(res: Option<ProcInfo>, expected: &str) -> bool {
    if let Some(res) = res {
        res.exit_code == Some(0)
            && res.error.unwrap_or_default().trim() == ""
            && res.output.unwrap_or_default().trim() == expected
    } else {
        false
    }
}

/// start the provided [Command] in a loop until it succeeds and provides the expected output on stdout, with no output on stderr.
pub async fn until_running(command: &mut Command, expected: &str) {
    while let res = collect_proc_info(command.pipe_stdout().pipe_stderr().spawn()).await
        && is_running(res, expected)
    {
        sleep(Duration::from_millis(10)).await;
    }
}
