use crate::prelude::Registry;
use crate::prelude::{Key, Value};
use regashii::ValueName;
use std::collections::BTreeMap;

/// A trait defining how to compute a diff between two items.
///
/// This trait is generic over a lifetime 'a, with an associated
/// Input type (the type that is diffed) and Output type (the difference result).
pub trait Diff<'a> {
    type Input: 'a;
    type Output;

    /// Compute the diff between two inputs and return the output.
    fn diff(this: Self::Input, other: Self::Input) -> Self::Output;
}

/// Combines two BTreeMaps (an "old" and a "new" version) by pairing
/// values with matching keys. For keys only in the old map, the new value is None;
/// and for keys only in the new map, the old value is None.
///
/// Returns a Vec of tuples, each containing an Option referencing a value from old and new.
pub fn combine<'a, 'b, K: std::cmp::Ord, V>(
    old: &'a BTreeMap<K, V>,
    new: &'b BTreeMap<K, V>,
) -> Vec<(Option<&'a V>, Option<&'b V>)> {
    let mut pairs: Vec<(Option<&V>, Option<&V>)> = Vec::new();

    // For every entry present in the old map, pair it with the corresponding value in the new map (if it exists)
    for (name, value) in old.iter() {
        pairs.push((Some(value), new.get(name)));
    }

    // For every entry in the new map that is not present in the old map, add a pair with None as the old value.
    for (name, value) in new.iter() {
        if !old.contains_key(name) {
            pairs.push((None, Some(value)));
        }
    }

    pairs
}

/// Enum representing possible operations for modifying registry values.
#[derive(Debug)]
pub enum Operation {
    Add {
        name: ValueName,
        value: regashii::Value,
    },
    Delete {
        name: ValueName,
    },
    Modify {
        name: ValueName,
        value: regashii::Value,
    },
}

impl<'a> Diff<'a> for Value {
    // The diff implementation for registry values compares two optional Value references.
    type Input = Option<&'a Value>;
    // The output is an optional Operation detailing what change should be applied.
    type Output = Option<Operation>;

    /// Computes the difference between two values.
    ///
    /// If a value exists in old but not in new, a Delete operation is generated.
    /// If a value exists in new but not in old, an Add operation is generated.
    /// If both exist but the values differ, a Modify operation is generated.
    /// If they are equal, returns None.
    fn diff(old: Self::Input, new: Self::Input) -> Self::Output {
        match (old, new) {
            (Some(old), None) => Some(Operation::Delete {
                name: old.name().clone(),
            }),
            (None, Some(new)) => Some(Operation::Add {
                name: new.name().clone(),
                value: new.value().clone(),
            }),
            (Some(old), Some(new)) if old != new => Some(Operation::Modify {
                name: new.name().clone(),
                value: new.value().clone(),
            }),
            _ => None,
        }
    }
}

impl<'a> Diff<'a> for Key {
    // The diff implementation for Keys compares two optional Key references.
    type Input = Option<&'a Key>;
    // The output is an optional Key that represents changes.
    type Output = Option<Key>;

    /// Computes the diff between two keys.
    ///
    /// This function compares the two keys:
    /// - If the key exists only in the old registry, it is marked as deleted.
    /// - If the key exists only in the new registry, it is marked as newly created.
    /// - If the key exists in both and there are differences in their values,
    ///   each value difference is computed and the resulting operations are applied.
    /// - If no differences are found, returns None.
    fn diff(this: Self::Input, other: Self::Input) -> Self::Output {
        match (this, other) {
            // The key is present in old, but missing in new, so mark it as deleted.
            (Some(old), None) => Some(Key::deleted(old.name().clone())),
            // The key is new.
            (None, Some(new)) => Some(new.clone()),
            // Both keys exist but they differ, so compute value differences.
            (Some(old), Some(new)) if old != new => {
                let ops: Vec<Operation> = combine(old.values(), new.values())
                    .into_iter()
                    .filter_map(|(old, new)| Value::diff(old, new))
                    .collect();

                let mut key = regashii::Key::new();
                for op in ops {
                    match op {
                        Operation::Add { name, value } => {
                            key = key.with(name, value);
                        }
                        Operation::Delete { name } => {
                            key = key.with(name, regashii::Value::Delete);
                        }
                        Operation::Modify { name, value } => {
                            key = key.with(name, value);
                        }
                    }
                }
                Some(Key::new(old.name().clone(), key))
            }
            _ => None,
        }
    }
}

impl<'a> Diff<'a> for Registry {
    // The diff implementation for Registries compares two Registry references.
    type Input = &'a Registry;
    // The output is a new regashii::Registry containing the diff.
    type Output = regashii::Registry;

    /// Computes the diff between two registries.
    ///
    /// This function iterates over the keys of both registries, calculates
    /// their individual differences, and then constructs a new registry patch containing all changes.
    fn diff(o_reg: Self::Input, n_reg: Self::Input) -> Self::Output {
        let mut patch = regashii::Registry::new(regashii::Format::Regedit4);

        let pairs = combine(o_reg.keys(), n_reg.keys());
        for (this, other) in pairs {
            if let Some(key) = Key::diff(this, other) {
                patch = patch.with(key.name().clone(), key.into());
            }
        }
        patch
    }
}

#[cfg(test)]
mod tests {
    use regashii::KeyKind;

    use super::*;
    use crate::prelude::Hive;

    fn generate_diff(hive: Hive) -> regashii::Registry {
        let o_reg = Registry::try_from("./registries/old.reg", hive).unwrap();
        let n_reg = Registry::try_from("./registries/new.reg", hive).unwrap();
        Registry::diff(&o_reg, &n_reg)
    }

    #[test]
    fn test_diff_delete_key() {
        let hive = Hive::LocalMachine;
        let diff = generate_diff(hive);

        let test_key = regashii::KeyName::new(format!("{}\\{}", hive, "TestKeyDelete"));
        let key = diff.keys().get(&test_key);
        assert!(key.is_some());
        let key = key.unwrap();
        assert_eq!(key.kind(), KeyKind::Delete);
        assert_eq!(key.values().len(), 0);
    }

    #[test]
    fn test_diff_create_key() {
        let hive = Hive::LocalMachine;
        let diff = generate_diff(hive);

        let test_key = regashii::KeyName::new(format!("{}\\{}", hive, "TestKeyCreate"));
        let key = diff.keys().get(&test_key);
        assert!(key.is_some());
        let key = key.unwrap();
        assert_eq!(key.kind(), KeyKind::Add);
    }

    #[test]
    fn test_diff_value_create() {
        let hive = Hive::LocalMachine;
        let diff = generate_diff(hive);

        let test_key = regashii::KeyName::new(format!("{}\\{}", hive, "TestValueCreate"));
        let key = diff.keys().get(&test_key);
        assert!(key.is_some());
        let key = key.unwrap();

        let value = key
            .values()
            .get(&regashii::ValueName::Named("CreateValue".to_string()));
        assert!(value.is_some());

        let value = value.unwrap();
        assert_eq!(value, &regashii::Value::Sz("new".to_string()));
    }

    #[test]
    fn test_diff_value_delete() {
        let hive = Hive::LocalMachine;
        let diff = generate_diff(hive);

        let test_key = regashii::KeyName::new(format!("{}\\{}", hive, "TestValueDelete"));
        let key = diff.keys().get(&test_key);
        assert!(key.is_some());
        let key = key.unwrap();

        let value = key
            .values()
            .get(&regashii::ValueName::Named("DeleteValue".to_string()));
        assert!(value.is_some());

        let value = value.unwrap();
        assert_eq!(value, &regashii::Value::Delete);
    }

    #[test]
    fn test_diff_value_update() {
        let hive = Hive::LocalMachine;
        let diff = generate_diff(hive);

        let test_key = regashii::KeyName::new(format!("{}\\{}", hive, "TestValueUpdate"));
        let key = diff.keys().get(&test_key);
        assert!(key.is_some());
        let key = key.unwrap();

        let value = key
            .values()
            .get(&regashii::ValueName::Named("TestValueUpdate".to_string()));
        assert!(value.is_some());

        let value = value.unwrap();
        assert_eq!(value, &regashii::Value::Sz("new".to_string()));
    }

    #[test]
    fn test_diff_no_change() {
        let hive = Hive::LocalMachine;
        let diff = generate_diff(hive);

        let test_key = regashii::KeyName::new(format!("{}\\{}", hive, "TestNoChange"));
        let key = diff.keys().get(&test_key);
        assert!(key.is_none());
    }
}
