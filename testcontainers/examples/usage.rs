extern crate testcontainers;

use testcontainers::{clients::DockerCli, *};

struct FakeImage {}

impl Image for FakeImage {
    type Args = Vec<String>;

    fn descriptor(&self) -> String {
        unimplemented!()
    }

    fn wait_until_ready<D: Docker>(&self, _container: &Container<D, Self>) {
        unimplemented!()
    }

    fn args(&self) -> Self::Args {
        unimplemented!()
    }

    fn with_args(self, _arguments: Self::Args) -> Self {
        unimplemented!()
    }
}

impl Default for FakeImage {
    fn default() -> Self {
        unimplemented!()
    }
}

struct FakeClient {}

impl ContainerClient<FakeImage> for FakeClient {
    fn new_container_client<D: Docker>(_container: &Container<D, FakeImage>) -> Self {
        unimplemented!()
    }
}

fn main() {
    let image = FakeImage {};

    let container = DockerCli::new().run(image);

    let _client = container.connect::<FakeClient>();
}
