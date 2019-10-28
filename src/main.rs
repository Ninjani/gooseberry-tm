#![allow(dead_code)]
#[macro_use]
extern crate lazy_static;

use anyhow::Error;

pub mod entry;
pub mod errors;
pub mod tabs;
pub mod utility;

fn main() -> Result<(), Error> {
    Ok(())
}
