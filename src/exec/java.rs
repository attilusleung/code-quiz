use std::time::Duration;
use super::CodeExec;
use super::docker::{dangling_container, run_in_container};

pub async fn exec_java_in_container(source: &str, image_name: &str, time: Duration, stdin: Option<&str>)
-> Result<CodeExec, anyhow::Error>
{
    let container = dangling_container(image_name, vec!["sh"], Duration::from_secs(2), None, false).await?;

    match run_in_container(&container, vec!["tee", "main.java"], Duration::from_secs(2), true, Some(source)).await {
        Ok(CodeExec::Executed(Some(0), _, _)) => {},
        _ => {
            anyhow::bail!("Failed to create java sourcecode")
        }
    }

    let ret = run_in_container(&container, vec!["java", "main.java"], time, false, stdin).await;
    ret
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_exec() {
        let ret = exec_java_in_container("public class Main{public static void main(String[] args){System.out.println(\"yeet\");}}", "test:latest", Duration::from_secs(10), None).await.unwrap();

        assert!(matches!(ret, CodeExec::Executed(Some(0), s, _) if s == "yeet\n"));

        let ret = exec_java_in_container(
            "import java.io.File;
            import java.io.IOException;
            public class Main{public static void main(String[] args){
                try{
                    (new File(\"yeet.txt\")).createNewFile();
                } catch (IOException e) {
                    System.out.println(\"yeet\");
                }
            }
            }",
            "test:latest", Duration::from_secs(10), None).await.unwrap();

        dbg!(&ret);

        assert!(matches!(ret, CodeExec::Executed(Some(0), s, _) if s == "yeet\n"));
    }
}
