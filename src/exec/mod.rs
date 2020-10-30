use anyhow::Error;
use tokio::process::Command;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;
use std::time::Duration;
use std::str;
use std::process::Stdio;

pub mod python;
pub mod docker;
pub mod java;

type Status = Option<i32>;


// Expected outcomes of successful code execution
// Ignores unexpected errors, such as being unable to start a process
#[derive(Debug, Eq, PartialEq)]
pub enum CodeExec {
    Executed(Status, String, String),
    Timeout,
}

async fn create_child<'a>(command: &str, args: impl IntoIterator<Item = &'a str>, stdin: Option<&str>) -> Result<tokio::process::Child, Error> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    match stdin {
        Some(s) => {
            let child = child.stdin.as_mut().unwrap();
            child.write_all(s.as_bytes()).await?;
            child.flush().await?;
        },
        None => {},
    }

    Ok(child)
}


pub async fn exec_timed<'a>(command: &str, args: impl IntoIterator<Item = &'a str>, time: Duration, stdin: Option<&str>)
-> Result<CodeExec, Error>
{
    let child = create_child(command, args, stdin).await?;

    let command = child.wait_with_output();
    let timed_command = timeout(time, command);
    let output = match timed_command.await {
        Err(_) => {
            return Ok(CodeExec::Timeout)
        },
        Ok(v) => v?
    };

    Ok(CodeExec::Executed(
            output.status.code(),
            str::from_utf8(&output.stdout)?.to_string(),
            str::from_utf8(&output.stderr)?.to_string()
        ))
}


pub async fn exec_dangling<'a>(command: &str, args: impl IntoIterator<Item = &'a str>, stdin: Option<&str>)
-> Result<tokio::process::Child, Error>
{
    create_child(command, args, stdin).await
}
