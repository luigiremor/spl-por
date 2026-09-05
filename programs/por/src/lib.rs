use anchor_lang::prelude::*;

pub mod errors;
pub mod state;

pub use errors::PorError;
pub use state::*;

declare_id!("6KpWoEs5mAdHGpVffKXd2JrE7PRjVa2YxQbCeRvzsa9Y");

#[program]
pub mod por {
    use super::*;
}
