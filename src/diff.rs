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
            (Some(old), Some(new)) => {
                let ops: Vec<Option<Operation>> = combine_values(old.values(), new.values())
                    .into_iter()
                    .map(|(old, new)| Value::diff(old, new))
                    .collect();

                if ops.is_empty() {
                    return None;
                }

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
