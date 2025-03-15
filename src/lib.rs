mod diff;
mod registry;

pub mod prelude {
    pub use crate::diff::{Diff, KeyOperation, Operation, ValueOperation};
    pub use crate::registry::{Key, Registry};
    pub use regashii::KeyName;
}
