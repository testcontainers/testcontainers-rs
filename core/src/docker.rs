use Container;
use Image;
use Logs;
use Ports;

pub trait Docker
where
    Self: Sized,
{
    fn run<I: Image>(&self, image: I) -> Container<Self, I>;
    fn logs(&self, id: &str) -> Logs;
    fn ports(&self, id: &str) -> Ports;
    fn rm(&self, id: &str);
    fn stop(&self, id: &str);
}
