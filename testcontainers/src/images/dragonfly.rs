use crate::{core::WaitFor, Image, ImageArgs};

const NAME: &str = "docker.dragonflydb.io/dragonflydb/dragonfly";
const TAG: &str = "latest";

#[derive(Debug, Default)]
pub struct Dragonfly;

impl Image for Dragonfly {
    type Args = DragonflyArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr(
            "AcceptServer - listening on port",
        )]
    }
}

#[derive(Debug, Clone, Default)]
pub struct DragonflyArgs {}

impl ImageArgs for DragonflyArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        let args = vec![
            "--alsologtostderr".to_owned(),
            "--logbuflevel=-1".to_owned(),
        ];
        Box::new(args.into_iter())
    }
}
