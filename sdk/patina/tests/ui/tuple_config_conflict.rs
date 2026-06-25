//! Test that a conflict inside a tuple parameter is rejected at compile time.

use patina::{
    component::{
        component,
        params::{Config, ConfigMut},
    },
    error::Result,
};

pub struct TestComponent;

#[component]
impl TestComponent {
    fn entry_point(self, _params: (ConfigMut<u32>, Config<u32>)) -> Result<()> {
        Ok(())
    }
}

fn main() {}
