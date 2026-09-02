use pinnacle_api::process::Child;
use tokio::io::{AsyncRead, AsyncReadExt};

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
