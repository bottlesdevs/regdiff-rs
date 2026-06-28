mod apply;
mod diff;
mod registry;

pub mod prelude {
    pub use crate::apply::{apply, apply_files};
    pub use crate::diff::Diff;
    pub use crate::registry::{Hive, Key, Registry, Value};
    pub use regashii::KeyName;
}
