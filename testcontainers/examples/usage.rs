extern crate testcontainers;

use testcontainers::{clients::DockerCli, *};

struct FakeImage {
    authentication_token: String,
}

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
        // Generate authentication information here
        FakeImage {
            authentication_token: unreachable!(),
        }
    }
}

struct FakeClient {}

fn main() {
    let image = FakeImage::default();

    let container = DockerCli::new().run(image);

    let _client = container.connect(|c| {
        // Query all the necessary information from our container so that you can connect to
        // it with our client, like host ports or authentication information

        let host_port = c.get_host_port(8080);
        let auth_token = c.image().authentication_token;

        FakeClient {}
    });
}
