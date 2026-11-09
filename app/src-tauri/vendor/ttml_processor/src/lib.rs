pub mod amll;
mod constants;
pub mod error;
mod generator;
pub mod model;
mod parser;
pub mod utils;

pub use generator::{
    GeneratorConfig,
    generate_ttml,
};
pub use parser::parse_ttml;
