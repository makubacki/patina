//! Test that a conflict inside an `Option<P>` parameter is rejected at compile time.

use patina::{
    component::{component, params::ConfigMut},
    error::Result,
};

pub struct TestComponent;

#[component]
impl TestComponent {
    fn entry_point(self, _config: ConfigMut<u32>, _maybe: Option<ConfigMut<u32>>) -> Result<()> {
        Ok(())
    }
}

fn main() {}
