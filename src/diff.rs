use crate::prelude::{Key, Registry, Value};
use regashii::{KeyName, ValueName};
use std::collections::BTreeMap;

/// Enum representing possible operations for modifying registry values.
#[derive(Debug)]
pub enum Operation<Data> {
    Unchanged,
    Add { data: Data },
    Delete { data: Data },
    Modify { old_data: Data, new_data: Data },
}

/// A trait defining how to compute a diff between two items.
///
/// This trait is generic over a lifetime 'a, with an associated
/// Input type (the type that is diffed) and Output type (the difference result).
pub trait Diff {
    type Input<'a>;
    type Output<'a>;

    fn diff<'a>(old: Self::Input<'a>, new: Self::Input<'a>) -> Self::Output<'a>;
}

/// Combines two BTreeMaps (an "old" and a "new" version) by pairing
/// values with matching keys. For keys only in the old map, the new value is None;
/// and for keys only in the new map, the old value is None.
///
/// Returns a Vec of tuples, each containing an Option referencing a value from old and new.
fn combine<'a, 'b, K: std::cmp::Ord, V>(
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

impl Diff for Value {
    type Input<'a> = Option<&'a Value>;
    type Output<'a> = Operation<&'a Value>;

    /// Computes the difference between two values.
    ///
    /// If a value exists in old but not in new, an [Operation::Delete] operation is generated.
    /// If a value exists in new but not in old, an [Operation::Add] operation is generated.
    /// If both exist but the values differ, a [Operation::Modify] operation is generated.
    /// Otherwise a [Operation::Unchanged] operation is generated.
    fn diff<'a>(old: Self::Input<'a>, new: Self::Input<'a>) -> Self::Output<'a> {
        match (old, new) {
            (Some(old), None) => Operation::Delete { data: old },
            (None, Some(new)) => Operation::Add { data: new },
            (Some(old), Some(new)) if old != new => Operation::Modify {
                old_data: old,
                new_data: new,
            },
            _ => Operation::Unchanged,
        }
    }
}

impl Operation<&Value> {
    fn to_value(self) -> Option<(ValueName, regashii::Value)> {
        match self {
            Operation::Add { data } => Some(data.clone().into_regashii_value()),
            Operation::Delete { data } => Some(data.clone().into_deleted_value()),
            Operation::Modify { new_data, .. } => Some(new_data.clone().into_regashii_value()),
            _ => None,
        }
    }
}

impl Diff for Key {
    type Input<'a> = Option<&'a Self>;
    type Output<'a> = Operation<Self>;

    /// Computes the diff between two keys.
    ///
    /// This function compares the two keys:
    /// - If the key exists only in the old registry, a [Operation::Delete] operation is generated.
    /// - If the key exists only in the new registry, a [Operation::Add] operation is generated.
    /// - If the key exists in both then:
    ///     - If the key names are different i.e the keys are not the same, a [Operation::Modify] operation is generated.
    ///     - If there are differences in their values, each value difference is computed and a [Operation::Add] operation is generated.
    /// - If no differences are found, a [Operation::Unchanged] operation is generated.
    fn diff<'a>(old: Self::Input<'a>, new: Self::Input<'a>) -> Self::Output<'a> {
        match (old, new) {
            (Some(old), None) => Operation::Delete { data: old.clone() },
            (None, Some(new)) => Operation::Add { data: new.clone() },
            (Some(old), Some(new)) if old.name() != new.name() => Operation::Modify {
                old_data: old.clone(),
                new_data: new.clone(),
            },
            (Some(old), Some(new)) if old != new => {
                let ops: Vec<Operation<&Value>> = combine(old.values(), new.values())
                    .into_iter()
                    .map(|(old, new)| Value::diff(old, new))
                    .collect();

                let mut key = regashii::Key::new();
                for op in ops {
                    if let Some((name, value)) = op.to_value() {
                        key = key.with(name, value);
                    }
                }
                Operation::Add {
                    data: Key::new(new.name().clone(), key),
                }
            }
            _ => Operation::Unchanged,
        }
    }
}

impl Operation<Key> {
    fn to_keys(self) -> Vec<(KeyName, regashii::Key)> {
        match self {
            Operation::Unchanged => Vec::new(),
            Operation::Add { data } => vec![data.into_regashii_key()],
            Operation::Delete { data } => vec![data.into_deleted_key()],
            Operation::Modify { old_data, new_data } => {
                vec![old_data.into_deleted_key(), new_data.into_regashii_key()]
            }
        }
    }
}

impl Diff for Registry {
    type Input<'a> = &'a Self;
    type Output<'a> = regashii::Registry;

    /// Computes the diff between two registries.
    ///
    /// This function iterates over the keys of both registries, calculates
    /// their individual differences, and then constructs a new registry patch containing all changes.
    fn diff<'a>(old: Self::Input<'a>, new: Self::Input<'a>) -> Self::Output<'a> {
        let mut patch = regashii::Registry::new(regashii::Format::Regedit4);

        let pairs = combine(old.keys(), new.keys());
        for (this, other) in pairs {
            for (name, key) in Key::diff(this, other).to_keys() {
                patch = patch.with(name, key);
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
