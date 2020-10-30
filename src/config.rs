use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use std::collections::HashMap;

// TODO: Mock out the entire config for tests

lazy_static!{
    pub static ref CONFIG: Config = {
        use toml::value::Value;
        use std::fs::read_to_string;

        let file_string = read_to_string("./config.toml")
            .expect("Failed to read toml");
        let mut timeout = 2000;
        let mut template: String = String::new();
        let mut questions: HashMap<String, Question> = HashMap::new();
        let mut max_proc = 5;
        let mut java_test_file = String::new();
        match toml::from_str(&file_string).expect("work") {
            toml::Value::Table(t) => {
                for (k, v) in t {
                    match (&k[..], &v) {
                        ("timeout", Value::Integer(i)) => {
                            timeout = *i as u32;
                        },
                        ("max_proc", Value::Integer(i)) => {
                            max_proc = *i as usize;
                        },
                        ("template", Value::String(s)) => {
                            template = s.to_owned();
                        },
                        ("java_test_file", Value::String(s)) => {
                            java_test_file = read_to_string(s).unwrap();
                        },
                        (_, Value::Table(_)) =>  {
                            let q: Question = v.try_into().unwrap();
                            questions.insert(q.handle.to_string(), q);
                        },
                        // ignore extra keys
                        (_, _) => {},
                    };
                }
            },
            _ => panic!("what"),
        }
        Config {timeout, template, questions, max_proc, java_test_file}
    };
}

pub struct Config {
    pub timeout: u32,
    pub template: String,
    pub java_test_file: String,
    pub questions: HashMap<String, Question>,
    pub max_proc: usize,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct Question {
    pub handle: String,
    pub function_name: String,
    pub prompt: String,
    pub python: Python,
    pub java: Java,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct Python {
    pub test_case: String,
    pub boilerplate: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct Java {
    pub test_case: String,
    pub boilerplate: String,
    pub func_call: String,
}

// Force evaluate lazy static
// This will panic if there is something wrong with CONFIG
pub fn verify_config() {
    CONFIG.timeout;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_questions() {
        // let new_hash = HashMap::new();
        let sample = Question {
            handle: String::new(),
            function_name: "identity".to_string(),
            prompt: String::new(),
            python: Python::default(),
            java: Java::default(),
        };
        let questions = &CONFIG.questions;
        assert_eq!(questions.get("identity").unwrap().function_name, sample.function_name);
    }

    #[test]
    fn test_get_language() {
        let sample_python = Python {
            test_case: "[([1], 1), ([2], 2), ([3], 3), ([-400], -400)]".to_string(),
            boilerplate: "def identity(x):\n    pass".to_string(),
        };

        assert_eq!(CONFIG.questions.get("identity").unwrap().python, sample_python);
    }
}
