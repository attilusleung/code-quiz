use std::time::Duration;
use std::str;
use log::error;
use std::process::{Command, Stdio};

use super::{CodeExec, exec_timed};

#[derive(Debug)]
pub struct DockerID {
    id: String
}

impl Drop for DockerID {
    fn drop(&mut self) {
        kill_container(&self.id);
    }
}

// TODO: Also I think it would be nice to disable syscalls
fn gen_args<'a>(image_name: &'a str, command: impl IntoIterator<Item=&'a str>, extra_args: impl IntoIterator<Item=&'a str>, readonly: bool) -> Vec<&str> {
    let mut args = vec!["run", "--rm", "-i",
        "--network", "none", "--user", "1000", "-m", "100m", "--memory-swap", "100m",
        "--kernel-memory", "100m"];

    args.extend(extra_args);

    if readonly {
        args.push("--read-only");
    }

    args.push(image_name);

    args.extend(command);

    args
}


pub async fn exec_in_container<'a>(
    image_name: &'a str,
    command: impl IntoIterator<Item=&'a str>,
    time: Duration,
    stdin: Option<&str>,
    readonly: bool
    ) -> Result<CodeExec, anyhow::Error> {

    let args = gen_args(image_name, command, vec!["-a", "STDIN", "-a", "STDOUT", "-a", "STDERR"], readonly);

    exec_timed(
        "docker",
        args,
        time,
        stdin
    ).await
}

pub async fn dangling_container<'a>(
    image_name: &'a str,
    command: impl IntoIterator<Item=&'a str>,
    time: Duration,
    stdin: Option<&str>,
    readonly: bool
) -> Result<DockerID, anyhow::Error> {
    let args = gen_args(image_name, command, vec!["-t", "-d"], readonly);

    match exec_timed("docker", args, time, stdin).await? {
        CodeExec::Executed(Some(0), stdout, _) => Ok(DockerID{id: stdout.trim().to_string()}),
        e => Err(anyhow::Error::msg(format!("Failed to start container: {:?}", e)))
    }
}

pub async fn run_in_container<'a>(
    container_id: &'a DockerID,
    command: impl IntoIterator<Item=&'a str>,
    time: Duration,
    root: bool,
    stdin: Option<&str>,
) -> Result<CodeExec, anyhow::Error>
{
    let mut args = vec!["exec", "-i"];

    if root {
        args.push("-u");
        args.push("0");
    }

    args.push(&container_id.id);
    args.extend(command);

    exec_timed("docker", args, time, stdin).await
}

fn kill_container(
    container_id: &str
)
{
    match Command::new("docker").arg("kill").arg(container_id)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
        Err(_) => error!("failed to kill docker container"),
        _ => {}
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::test::Test;
    use serde_json::to_string;

    #[tokio::test]
    async fn test_exec_in_container() {

        let res = exec_in_container(
            "test:latest",
            vec!["python", "./test.py", "yeet"],
            Duration::from_secs(2),
            Some(&to_string(&Test{
                code: "def yeet(): return True",
                test_case: "[([], True)]"}
                ).unwrap()),
            true,
        ).await.unwrap();

        assert_eq!(
            CodeExec::Executed(
                Some(0),
                "All test cases passed!".to_string(),
                String::new()
                ),
            res
        )
    }

    #[tokio::test]
    async fn test_exec_in_container_sec() {
        // No network permissions
        let res = exec_in_container(
            "test:latest",
            vec!["ping", "-c", "5", "8.8.8.8"],
            Duration::from_secs(2),
            None,
            true,
        ).await.unwrap();

        assert!(matches!(
            res,
            CodeExec::Executed(Some(x), _, _) if x != 0
        ));

        // No network permissions
        // let res = exec_in_container(
        //     "test:latest",
        //     vec!["python", "./test.py" "ping"],
        //     Duration::from_secs(2),
        //     Some(&to_string(&Test{
        //         code: "def ping(): import os; return os.system('ping 8.8.8.8') != 0",
        //         test_case: "[([], True)]"}
        //         ).unwrap())
        // ).await.unwrap();
        //
        // assert_eq!(
        //     CodeExec::Executed(
        //         Some(0),
        //         "All test cases passed!".to_string(),
        //         "connect: Network is unreachable\n".to_string(),
        //         ),
        //     res
        // );

        // No write permissions
        let res = exec_in_container(
            "test:latest",
            vec!["touch", "test.py"],
            Duration::from_secs(2),
            None,
            true,
        ).await.unwrap();

        dbg!(&res);

        assert!(matches!(res, CodeExec::Executed(Some(x), _, _) if x != 0));

        // No write permissions
        // let res = exec_in_container(
        //     "test:latest",
        //     vec!["python", "./test.py", "write"],
        //     Duration::from_secs(2),
        //     Some(&to_string(&Test{
        //         code: "def write(): open('test.txt', 'w').write('this shouldnt work')",
        //         test_case: "[([], None)]"}
        //         ).unwrap())
        // ).await.unwrap();
        //
        // assert!(matches!(res, CodeExec::Executed(Some(1), _, _)));

        // Fork bomb
        let res = exec_in_container(
            "test:latest",
            vec!["python", "-c", "import os\nwhile True:\n  os.fork()"],
            Duration::from_secs(10),
            None,
            true,
        ).await.unwrap();

        dbg!(&res);

        assert!(matches!(
                res, CodeExec::Executed(Some(x), _, _) if x != 0
        ));

        // test root
        let res = exec_in_container(
            "test:latest",
            vec!["chroot", "/usr"],
            Duration::from_secs(10),
            None,
            true,
        ).await.unwrap();

        dbg!(&res);

        assert!(matches!(
                res, CodeExec::Executed(Some(x), _, _) if x != 0
        ));

        let res = exec_in_container(
            "test:latest",
            vec!["truncate", "-s", "500M", "garbage.txt"],
            Duration::from_secs(10),
            None,
            false,
        ).await.unwrap();

        dbg!(&res);

        assert!(matches!(
                res, CodeExec::Executed(Some(x), _, _) if x != 0
        ));
    }

    #[tokio::test]
    async fn test_dangling_container() {
        let res = dangling_container("test:latest", vec!["sh"], Duration::from_secs(10), None, false).await.unwrap();
        dbg!(&res);
    }
}
