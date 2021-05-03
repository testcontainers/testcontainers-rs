use crate::{core::WaitFor, Image};

const CONTAINER_IDENTIFIER: &str = "redis";
const DEFAULT_TAG: &str = "5.0";

#[derive(Debug, Default, Clone)]
pub struct RedisArgs;

impl IntoIterator for RedisArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct Redis {
    tag: String,
    arguments: RedisArgs,
}

impl Default for Redis {
    fn default() -> Self {
        Redis {
            tag: DEFAULT_TAG.to_string(),
            arguments: RedisArgs {},
        }
    }
}

impl Image for Redis {
    type Args = RedisArgs;

    fn descriptor(&self) -> String {
        format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Ready to accept connections")]
    }
}

impl Redis {
    pub fn with_tag(self, tag_str: &str) -> Self {
        Redis {
            tag: tag_str.to_string(),
            ..self
        }
    }
}
