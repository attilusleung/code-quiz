use crate::exec::CodeExec;
use crate::exec::python::exec_python;
use crate::exec::java::exec_java_in_container;
use crate::exec::docker::exec_in_container;
use serde::Serialize;
use std::time::Duration;


// TODO: Make this not pub
#[derive(Serialize)]
pub struct Test<'a> {
    pub code: &'a str,
    pub test_case: &'a str,
}


// TODO: maybe want to make an enum or something for errors vs failures
pub fn gen_reply(success:bool, message:&str) -> String {
    use serde_json::json;
    json!({"success": success, "msg": message}).to_string()
}

pub async fn test_python(code: &str, func_name: &str, test_case: &str, time: Duration) -> String {

    use serde_json::to_string;

    // TODO: error handling
    let input = to_string(&Test{code, test_case}).unwrap();

    // TODO: Read test file from config
    match exec_python(vec!["./test.py", func_name], time, Some(&input)).await {
        Ok(CodeExec::Executed(Some(0), stdout, _)) => gen_reply(true, &stdout),
        Ok(CodeExec::Executed(Some(1), _, stderr)) => gen_reply(false, &stderr),
        Ok(CodeExec::Executed(Some(2), stdout, _)) => gen_reply(false, &stdout),
        Ok(CodeExec::Timeout) => gen_reply(false, "Test timed out."),
        _ => gen_reply(false, "A server error occured. Try again later."),
    }
}


pub async fn test_python_in_container(code: &str, func_name: &str, test_case: &str, time: Duration) -> String {

    use serde_json::to_string;

    // TODO: error handling
    let input = to_string(&Test{code, test_case}).unwrap();

    // TODO: Read test file from config
    // TODO: Generate everything from test file
    match exec_in_container("test", vec!["python", "./test.py", func_name], time, Some(&input), true).await {
        Ok(CodeExec::Executed(Some(0), stdout, _)) => gen_reply(true, &stdout),
        Ok(CodeExec::Executed(Some(1), _, stderr)) => gen_reply(false, &stderr),
        Ok(CodeExec::Executed(Some(2), stdout, _)) => gen_reply(false, &stdout),
        Ok(CodeExec::Timeout) => gen_reply(false, "Test timed out."),
        _ => gen_reply(false, "A server error occured. Try again later."),
    }
}


pub async fn test_java_in_container(code: &str, template: &str, func_call: &str, test_case: &str, time: Duration) -> String {

    let mut new_source = template.to_string();
    new_source.push_str(code);
    let new_source = new_source.replace("{{test_case}}", test_case);
    let new_source = new_source.replace("{{func_call}}", func_call);

    match exec_java_in_container(&new_source, "test", time, None).await {
        Ok(CodeExec::Executed(Some(0), stdout, _)) => gen_reply(true, &stdout),
        Ok(CodeExec::Executed(Some(1), _, stderr)) => gen_reply(false, &stderr),
        Ok(CodeExec::Executed(Some(2), stdout, _)) => gen_reply(false, &stdout),
        Ok(CodeExec::Timeout) => gen_reply(false, "Test timed out."),
        _ => gen_reply(false, "A server error occured. Try again later."),
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[tokio::test]
    async fn test_test_python() {
        let function = "
def solution():
    return True
        ".trim();

        let success = test_python(function, "solution", "[([], True)]", Duration::new(10, 0)).await;

        assert_eq!(success, gen_reply(true, "All test cases passed!"));

        let fail = test_python(function, "solution", "[([], False)]", Duration::new(10, 0)).await;

        assert_eq!(fail, gen_reply(false, "Test case failed on input `[]`: Expected \n`False`\nbut got \n`True`"));

        let fail = test_python("while True:\n    pass", "solution", "[([], False)]", Duration::new(10, 0)).await;

        assert_eq!(fail, gen_reply(false, "Test timed out."));

        let function = "
i = 0
def solution():
    global i
    i += 1
    return i
        ".trim();

        let global_test = test_python(function, "solution", "[([], 1), ([], 1)]", Duration::new(10, 0)).await;

        assert_eq!(global_test, gen_reply(true, "All test cases passed!"));
    }
}
