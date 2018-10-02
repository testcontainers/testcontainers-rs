use Container;
use Docker;

pub trait Image
where
    Self: Sized + Default,
    Self::Args: IntoIterator<Item = String>,
{
    type Args;

    fn descriptor(&self) -> String;
    fn wait_until_ready<D: Docker>(&self, container: &Container<D, Self>);
    fn args(&self) -> Self::Args;

    fn with_args(self, arguments: Self::Args) -> Self;
}
