use std::time::Duration;

use futures::{TryFutureExt, future::OptionFuture};
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
    futures::future::ready(fd.ok_or(()))
        .and_then(|handle| {
            let mut o = String::new();
            async move {
                handle
                    .read_to_string(&mut o)
                    .map_err(|_| ())
                    .await
                    .map(|_| o)
            }
        })
        .await
        .ok()
}

pub async fn collect_proc_info(child: Option<Child>) -> Option<ProcInfo> {
    OptionFuture::from(child.map(|mut child| async {
        ProcInfo {
            output: read_fd(child.stdout.as_mut()).await,
            error: read_fd(child.stderr.as_mut()).await,
            exit_code: child.wait_async().await.exit_code,
        }
    }))
    .await
}

/// we can test if a daemon is running with a trial command, with expected output.
/// the daemon is running if all of the following are true:
/// 1. the trial command gave a zero exit code
/// 2. the command produced no stderr output
/// 3. the command produced exactly the expected output on stdout (after trimming)
pub fn is_running(res: Option<ProcInfo>, expected: &str) -> bool {
    res.is_some_and(|res| {
        res.exit_code == Some(0)
            && res.error.unwrap_or_default().trim() == ""
            && res.output.unwrap_or_default().trim() == expected
    })
}

/// start the provided [Command] in a loop until it succeeds and provides the expected output on stdout, with no output on stderr.
pub async fn until_running(command: &mut Command, expected: &str) {
    while let res = collect_proc_info(command.pipe_stdout().pipe_stderr().spawn()).await
        && is_running(res, expected)
    {
        sleep(Duration::from_millis(10)).await;
    }
}
