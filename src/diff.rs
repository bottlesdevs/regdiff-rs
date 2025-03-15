use crate::{
    prelude::Registry,
    registry::{SharedKey, Value},
};
use regashii::{KeyName, ValueName};

#[derive(Debug)]
pub enum Operation<Name, Data> {
    Add { name: Name, data: Data },
    Delete { name: Name },
    Update { name: Name, new: Data },
}

pub type ValueOperation = Operation<ValueName, regashii::Value>;
pub type KeyOperation = Operation<KeyName, regashii::Key>;

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
        let name = o.borrow().name().clone();
        let matching_new = new.iter().find(|&n| n.borrow().name() == &name).cloned();
        pairs.push((Some(o.clone()), matching_new));
    }

    for n in new.iter() {
        let name = n.borrow().name().clone();
        if !old.iter().any(|o| o.borrow().name() == &name) {
            pairs.push((None, Some(n.clone())));
        }
    }

    pairs
}

pub fn combine_values(old: &[Value], new: &[Value]) -> Vec<(Option<Value>, Option<Value>)> {
    let mut pairs: Vec<(Option<Value>, Option<Value>)> = Vec::new();

    for o in old.iter() {
        let name = o.name().clone();
        let matching_new = new.iter().find(|&n| n.name() == &name).cloned();
        pairs.push((Some(o.clone()), matching_new));
    }

    for n in new.iter() {
        let name = n.name().clone();
        if !old.iter().any(|o| o.name() == &name) {
            pairs.push((None, Some(n.clone())));
        }
    }

    pairs
}

impl Diff for Value {
    type Output = Option<ValueOperation>;
    fn diff(this: Option<&Self>, other: Option<&Self>) -> Self::Output {
        match (this, other) {
            (Some(old), None) => Some(ValueOperation::Delete {
                name: old.name().clone(),
            }),
            (None, Some(new)) => Some(ValueOperation::Add {
                name: new.name().clone(),
                data: new.value().clone(),
            }),
            (Some(old), Some(new)) if old != new => Some(ValueOperation::Update {
                name: old.name().clone(),
                new: new.value().clone(),
            }),
            _ => None,
        }
    }
}

impl Diff for SharedKey {
    type Output = Vec<KeyOperation>;
    fn diff(this: Option<&Self>, other: Option<&Self>) -> Self::Output {
        match (this, other) {
            (Some(old), None) => vec![KeyOperation::Delete {
                name: old.borrow().name().clone(),
            }],
            (None, Some(new)) => vec![KeyOperation::Add {
                name: new.borrow().name().clone(),
                data: new.borrow().inner().clone(),
            }],
            (Some(old_key), Some(new_key)) => {
                let mut operations = vec![];
                let old_values: Vec<Value> = old_key
                    .borrow()
                    .values()
                    .iter()
                    .map(|(_, v)| v.clone())
                    .collect();
                let new_values: Vec<Value> = new_key
                    .borrow()
                    .values()
                    .iter()
                    .map(|(_, v)| v.clone())
                    .collect();

                combine_values(&old_values, &new_values)
                    .into_iter()
                    .for_each(|(old_val, new_val)| {
                        if let Some(op) = Value::diff(old_val.as_ref(), new_val.as_ref()) {
                            let op = match op {
                                Operation::Add { name, data } => KeyOperation::Add {
                                    name: old_key.borrow().name().clone(),
                                    data: regashii::Key::new().with(name, data),
                                },
                                Operation::Delete { name } => KeyOperation::Update {
                                    name: old_key.borrow().name().clone(),
                                    new: regashii::Key::new().with(name, regashii::Value::Delete),
                                },
                                Operation::Update { name, new } => KeyOperation::Update {
                                    name: old_key.borrow().name().clone(),
                                    new: regashii::Key::new().with(name, new),
                                },
                            };
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
