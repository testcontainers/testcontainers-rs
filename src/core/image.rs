use std::{env::var, fmt::Debug, time::Duration};

/// Represents a docker image.
///
/// Implementations are required to implement Default. The default instance of an [`Image`]
/// should have a meaningful configuration! It should be possible to [`run`][docker_run] the default
/// instance of an Image and get back a working container!
///
/// [`Image`]: trait.Image.html
/// [docker_run]: trait.Docker.html#tymethod.run
pub trait Image
where
    Self: Sized,
    Self::Args: IntoIterator<Item = String> + Debug + Clone,
{
    /// A type representing the arguments for an Image.
    ///
    /// There are a couple of things regarding the arguments of images:
    ///
    /// 1. Similar to the Default implementation of an Image, the Default instance
    /// of its arguments should be meaningful!
    /// 2. Implementations should be conservative about which arguments they expose. Many times,
    /// users will either go with the default arguments or just override one or two. When defining
    /// the arguments of your image, consider that the whole purpose is to facilitate integration
    /// testing. Only expose those that actually make sense for this case.
    type Args;

    /// The descriptor of the docker image.
    ///
    /// This should return a full-qualified descriptor.
    /// Implementations are encouraged to include a tag that will not change (i.e. NOT latest)
    /// in order to prevent test code from randomly breaking because the underlying docker
    /// suddenly changed.
    fn descriptor(&self) -> String;

    /// Returns a list of conditions that need to be met before a started container is considered ready.
    ///
    /// This method is the **🍞 and butter** of the whole testcontainers library. Containers are
    /// rarely instantly available as soon as they are started. Most of them take some time to boot
    /// up.
    ///
    /// The conditions returned from this method are evaluated **in the order** they are returned. Therefore
    /// you most likely want to start with a [`WaitFor::StdOutMessage`] or [`WaitFor::StdErrMessage`] and
    /// potentially follow up with a [`WaitFor::Duration`] in case the container usually needs a little
    /// more time before it is ready.
    fn ready_conditions(&self) -> Vec<WaitFor>;

    /// There are a couple of things regarding the environment variables of images:
    ///
    /// 1. Similar to the Default implementation of an Image, the Default instance
    /// of its environment variables should be meaningful!
    /// 2. Implementations should be conservative about which environment variables they expose. Many times,
    /// users will either go with the default ones or just override one or two. When defining
    /// the environment variables of your image, consider that the whole purpose is to facilitate integration
    /// testing. Only expose those that actually make sense for this case.
    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(std::iter::empty())
    }

    /// There are a couple of things regarding the volumes of images:
    ///
    /// 1. Similar to the Default implementation of an Image, the Default instance
    /// of its volumes should be meaningful!
    /// 2. Implementations should be conservative about which volumes they expose. Many times,
    /// users will either go with the default ones or just override one or two. When defining
    /// the volumes of your image, consider that the whole purpose is to facilitate integration
    /// testing. Only expose those that actually make sense for this case.
    fn volumes(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(std::iter::empty())
    }

    /// Returns the entrypoint this instance was created with.
    fn entrypoint(&self) -> Option<String> {
        None
    }
}

pub trait ImageExt: Image {
    fn with_args(self, args: Self::Args) -> RunnableImage<Self> {
        RunnableImage {
            image: self,
            image_args: args,
            name: None,
            network: None,
            ports: None,
        }
    }

    fn with_network(self, network: impl Into<String>) -> RunnableImage<Self>
    where
        Self::Args: Default,
    {
        RunnableImage {
            image: self,
            image_args: Self::Args::default(),
            name: None,
            network: Some(network.into()),
            ports: None,
        }
    }

    fn with_name(self, name: impl Into<String>) -> RunnableImage<Self>
    where
        Self::Args: Default,
    {
        RunnableImage {
            image: self,
            image_args: Self::Args::default(),
            name: Some(name.into()),
            network: None,
            ports: None,
        }
    }

    fn with_mapped_port<P: Into<Port>>(self, port: P) -> RunnableImage<Self>
    where
        Self::Args: Default,
    {
        RunnableImage {
            image: self,
            image_args: Self::Args::default(),
            name: None,
            network: None,
            ports: Some(vec![port.into()]),
        }
    }
}

impl<I> ImageExt for I where I: Image {}

/// Represents a port mapping between a local port and the internal port of a container.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    pub local: u16,
    pub internal: u16,
}

/// Represents a condition that needs to be met before a container is considered ready.
#[derive(Debug, PartialEq, Clone)]
pub enum WaitFor {
    /// An empty condition. Useful for default cases or fallbacks.
    Nothing,
    /// Wait for a message on the stdout stream of the container's logs.
    StdOutMessage { message: String },
    /// Wait for a message on the stderr stream of the container's logs.
    StdErrMessage { message: String },
    /// Wait for a certain amount of time.
    Duration { length: Duration },
}

impl WaitFor {
    pub fn message_on_stdout<S: Into<String>>(message: S) -> WaitFor {
        WaitFor::StdOutMessage {
            message: message.into(),
        }
    }

    pub fn message_on_stderr<S: Into<String>>(message: S) -> WaitFor {
        WaitFor::StdErrMessage {
            message: message.into(),
        }
    }

    pub fn seconds(length: u64) -> WaitFor {
        WaitFor::Duration {
            length: Duration::from_secs(length),
        }
    }

    pub fn millis(length: u64) -> WaitFor {
        WaitFor::Duration {
            length: Duration::from_millis(length),
        }
    }

    pub fn millis_in_env_var(name: &'static str) -> WaitFor {
        let additional_sleep_period = var(name).map(|value| value.parse());

        (|| {
            let length = additional_sleep_period.ok()?.ok()?;

            Some(WaitFor::Duration {
                length: Duration::from_millis(length),
            })
        })()
        .unwrap_or(WaitFor::Nothing)
    }
}

impl From<(u16, u16)> for Port {
    fn from((local, internal): (u16, u16)) -> Self {
        Port { local, internal }
    }
}

#[derive(Debug)]
pub struct RunnableImage<I: Image> {
    pub(crate) image: I,
    pub(crate) image_args: I::Args,
    pub(crate) name: Option<String>,
    pub(crate) network: Option<String>,
    pub(crate) ports: Option<Vec<Port>>,
}

impl<I: Image> RunnableImage<I> {
    pub fn image(&self) -> &I {
        &self.image
    }

    pub fn image_args(&self) -> &I::Args {
        &self.image_args
    }
}

impl<I: Image> RunnableImage<I> {
    pub fn with_image_args(self, args: <I as Image>::Args) -> RunnableImage<I> {
        Self {
            image_args: args,
            ..self
        }
    }

    pub fn with_name(self, name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..self
        }
    }

    pub fn with_network(self, network: impl Into<String>) -> Self {
        Self {
            network: Some(network.into()),
            ..self
        }
    }

    pub fn with_mapped_port<P: Into<Port>>(self, port: P) -> Self {
        let mut ports = self.ports.unwrap_or_default();
        ports.push(port.into());

        Self {
            ports: Some(ports),
            ..self
        }
    }
}

impl<I> From<I> for RunnableImage<I>
where
    I: Image + Default,
    <I as Image>::Args: Default,
{
    fn from(image: I) -> Self {
        Self {
            image,
            image_args: I::Args::default(),
            name: None,
            network: None,
            ports: None,
        }
    }
}
