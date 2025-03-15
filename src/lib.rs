mod diff;
mod registry;

pub mod prelude {
    pub use crate::diff::{Diff, Operation};
    pub use crate::registry::{Hive, Key, Registry};
    pub use regashii::KeyName;
}
