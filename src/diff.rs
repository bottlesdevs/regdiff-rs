use crate::prelude::Registry;
use crate::prelude::{Key, Value};
use regashii::{KeyName, ValueName};
use std::collections::BTreeMap;

pub trait Diff<'a> {
    type Input: 'a;
    type Output;
    fn diff(this: Self::Input, other: Self::Input) -> Self::Output;
}

pub fn combine_keys<'a, 'b>(
    old: &'a BTreeMap<KeyName, Key>,
    new: &'b BTreeMap<KeyName, Key>,
) -> Vec<(Option<&'a Key>, Option<&'b Key>)> {
    let mut pairs: Vec<(Option<&Key>, Option<&Key>)> = Vec::new();

    for (name, value) in old.iter() {
        pairs.push((Some(value), new.get(name)));
    }

    for (name, value) in new.iter() {
        if !old.contains_key(name) {
            pairs.push((None, Some(value)));
        }
    }

    pairs
}

pub fn combine_values<'a, 'b>(
    old: &'a BTreeMap<ValueName, Value>,
    new: &'b BTreeMap<ValueName, Value>,
) -> Vec<(Option<&'a Value>, Option<&'b Value>)> {
    let mut pairs: Vec<(Option<&Value>, Option<&Value>)> = Vec::new();

    for (name, value) in old.iter() {
        pairs.push((Some(value), new.get(name)));
    }

    for (name, value) in new.iter() {
        if !old.contains_key(name) {
            pairs.push((None, Some(value)));
        }
    }

    pairs
}

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
    type Input = Option<&'a Value>;
    type Output = Option<Operation>;
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
    type Input = Option<&'a Key>;
    type Output = Option<Key>;
    fn diff(this: Self::Input, other: Self::Input) -> Self::Output {
        match (this, other) {
            (Some(old), None) => Some(Key::deleted(old.name().clone())),
            (None, Some(new)) => Some(new.clone()),
            (Some(old), Some(new)) if old != new => {
                let ops: Vec<Option<Operation>> = combine_values(old.values(), new.values())
                    .into_iter()
                    .map(|(old, new)| Value::diff(old, new))
                    .filter(|op| op.is_some())
                    .collect();

                let mut key = regashii::Key::new();
                for op in ops {
                    match op {
                        Some(Operation::Add { name, value }) => {
                            key = key.with(name, value);
                        }
                        Some(Operation::Delete { name }) => {
                            key = key.with(name, regashii::Value::Delete);
                        }
                        Some(Operation::Modify { name, value }) => {
                            key = key.with(name, value);
                        }
                        None => {}
                    }
                }
                Some(Key::new(old.name().clone(), key))
            }
            _ => None,
        }
    }
}

impl<'a> Diff<'a> for Registry {
    type Input = &'a Registry;
    type Output = regashii::Registry;
    fn diff(o_reg: Self::Input, n_reg: Self::Input) -> Self::Output {
        let mut patch = regashii::Registry::new(regashii::Format::Regedit4);

        let pairs = combine_keys(o_reg.keys(), n_reg.keys());
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
