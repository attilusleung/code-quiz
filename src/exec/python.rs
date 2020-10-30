use std::time::Duration;
use std::str;

use super::{CodeExec, exec_timed};


// Run python code
pub async fn exec_python<'a, I>(args: I, time: Duration, stdin: Option<&str>) -> Result<CodeExec, anyhow::Error>
where I: IntoIterator<Item = &'a str>,
{
    exec_timed("python", args, time, stdin).await
}

#[cfg(test)]
mod tests{
    use super::*;

    #[tokio::test]
    async fn test_exec_python() {
        let success = exec_python(vec!["-c", "import sys; sys.exit()"], Duration::new(10, 0), None).await
        .expect("Something went wrong");

        assert_eq!(success, CodeExec::Executed(Some(0), "".to_string(), "".to_string()));

        let timeout = exec_python(vec!["-c", "import sys; sys.exit()"], Duration::new(0, 0), None).await
        .expect("Something went wrong");

        assert_eq!(timeout, CodeExec::Timeout);

        let bad_status = exec_python(vec!["-c", "import sys; sys.exit(1)"], Duration::new(10, 0), None).await
        .expect("Something went wrong");

        assert_eq!(bad_status, CodeExec::Executed(Some(1), "".to_string(), String::new()));

        let inf_code = "while True:\n    pass
        ";

        let inf_loop = exec_python(vec!["-c", inf_code], Duration::new(1, 0), None).await
        .expect("Something went wrong");

        assert_eq!(inf_loop, CodeExec::Timeout);

        let stdout = exec_python(vec!["-c", "print('yeet')"], Duration::new(10, 0), None).await
        .expect("Something went wrong");

        assert_eq!(stdout, CodeExec::Executed(Some(0), "yeet\n".to_string(), String::new()));

        let stdin = exec_python(vec!["-c", "print(input())"], Duration::new(10, 0), Some("yeet")).await
        .expect("Something went wrong");

        assert_eq!(stdin, CodeExec::Executed(Some(0), "yeet\n".to_string(), String::new()));
    }

}
