use crate::{core::WaitFor, Image};

const NAME: &str = "rabbitmq";
const TAG: &str = "3.8.22-management";

#[derive(Debug, Default, Clone)]
pub struct RabbitMq;

impl Image for RabbitMq {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout(
            "Server startup complete; 4 plugins started.",
        )]
    }
}
