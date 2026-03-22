mod error;
mod matcher;
mod service;
mod types;

pub use error::AuthorizeError;
pub use service::authorize_ref_update;
pub use types::{
    AuthorizeOptions, AuthorizeProfileResult, AuthorizeProfileState, AuthorizeReport, MatchedRule,
};
