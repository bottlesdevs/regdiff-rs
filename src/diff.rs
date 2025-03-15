use std::collections::BTreeMap;

use crate::{
    prelude::Registry,
    registry::{SharedKey, Value},
};

#[derive(Debug)]
pub enum Operation {
    Add {
        name: regashii::KeyName,
        data: regashii::Key,
    },
    Delete {
        name: regashii::KeyName,
    },
    Update {
        name: regashii::KeyName,
        new: regashii::Key,
    },
}

pub trait Diff {
    type Output;
    fn diff(this: Option<&Self>, other: Option<&Self>) -> Self::Output;
}

pub fn combine_child_keys(
    old: &[SharedKey],
    new: &[SharedKey],
) -> Vec<(Option<SharedKey>, Option<SharedKey>)> {
    let mut pairs: Vec<(Option<SharedKey>, Option<SharedKey>)> = Vec::new();

    for o in old.iter() {
        let name = o.borrow().path().clone();
        let matching_new = new.iter().find(|&n| n.borrow().path() == &name).cloned();
        pairs.push((Some(o.clone()), matching_new));
    }

    for n in new.iter() {
        let name = n.borrow().path().clone();
        if !old.iter().any(|o| o.borrow().path() == &name) {
            pairs.push((None, Some(n.clone())));
        }
    }

    pairs
}

pub fn combine_values<'a, 'b>(
    old: &'a BTreeMap<regashii::ValueName, Value>,
    new: &'b BTreeMap<regashii::ValueName, Value>,
) -> Vec<(Option<&'a Value>, Option<&'b Value>)> {
    let mut pairs: Vec<(Option<&'a Value>, Option<&'b Value>)> = Vec::new();

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

impl Diff for Value {
    type Output = Option<Operation>;
    fn diff(this: Option<&Self>, other: Option<&Self>) -> Self::Output {
        match (this, other) {
            (Some(old), None) => Some(Operation::Update {
                name: old.key_name().clone(),
                new: regashii::Key::new().with(old.name().clone(), regashii::Value::Delete),
            }),
            (None, Some(new)) => Some(Operation::Add {
                name: new.key_name().clone(),
                data: regashii::Key::new().with(new.name().clone(), new.value().clone()),
            }),
            (Some(old), Some(new)) if old != new => Some(Operation::Update {
                name: old.key_name().clone(),
                new: regashii::Key::new().with(old.name().clone(), new.value().clone()),
            }),
            _ => None,
        }
    }
}

impl Diff for SharedKey {
    type Output = Vec<Operation>;
    fn diff(this: Option<&Self>, other: Option<&Self>) -> Self::Output {
        match (this, other) {
            (Some(old), None) => vec![Operation::Delete {
                name: old.borrow().path().clone(),
            }],
            (None, Some(new)) => vec![Operation::Add {
                name: new.borrow().path().clone(),
                data: new.borrow().inner(),
            }],
            (Some(old_key), Some(new_key)) => {
                let mut operations = vec![];
                let old_key_ref = old_key.borrow();
                let new_key_ref = new_key.borrow();

                combine_values(old_key_ref.values(), new_key_ref.values())
                    .into_iter()
                    .for_each(|(old_val, new_val)| {
                        if let Some(op) = Value::diff(old_val, new_val) {
                            operations.push(op);
                        }
                    });

                // Recursively diff children
                let old_children = old_key.borrow().children();
                let new_children = new_key.borrow().children();

                combine_child_keys(&old_children, &new_children)
                    .into_iter()
                    .for_each(|(old, new)| {
                        operations.extend(SharedKey::diff(old.as_ref(), new.as_ref()));
                    });

                operations
            }
            _ => Vec::new(),
        }
    }
}

impl Diff for Registry {
    type Output = regashii::Registry;
    fn diff(this: Option<&Self>, other: Option<&Self>) -> Self::Output {
        let mut patch = regashii::Registry::new(regashii::Format::Regedit4);

        if this.is_none() {
            return patch;
        }

        if other.is_none() {
            return patch;
        }

        let o_reg = this.unwrap();
        let n_reg = other.unwrap();

        let diff = Diff::diff(Some(&o_reg.root()), Some(&n_reg.root()));

        for op in diff {
            patch = match op {
                Operation::Add { name, data } => patch.with(name, data),
                Operation::Delete { name } => patch.with(name, regashii::Key::deleted()),
                Operation::Update { name, new } => patch.with(name, new),
            };
        }

        patch
    }
}
