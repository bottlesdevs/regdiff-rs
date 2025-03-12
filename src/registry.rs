use regashii::ValueName;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

pub type SharedKey = Rc<RefCell<Key>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Unchanged,
    Inserted,
    Deleted,
    Updated,
}

#[derive(Debug)]
pub struct Value {
    name: ValueName,
    value: regashii::Value,
    status: Status,
}

impl Value {
    pub fn from(name: ValueName, value: regashii::Value) -> Self {
        Self {
            name,
            value,
            status: Status::Unchanged,
        }
    }
}

#[derive(Debug)]
pub struct Key {
    name: regashii::KeyName,
    parent: Option<Weak<RefCell<Key>>>,
    children: Vec<SharedKey>,
    values: BTreeMap<ValueName, Value>,
    status: Status,
}

impl Key {
    pub fn new(name: regashii::KeyName, inner: regashii::Key) -> Self {
        let values = inner
            .values()
            .iter()
            .map(|(name, value)| (name.clone(), Value::from(name.clone(), value.clone())))
            .collect();

        Self {
            name,
            parent: None,
            values,
            children: Vec::new(),
            status: Status::Unchanged,
        }
    }

    pub fn add_child(&mut self, child: SharedKey) {
        self.children.push(child);
    }

    pub fn name(&self) -> &regashii::KeyName {
        &self.name
    }

    pub fn values(&self) -> &BTreeMap<ValueName, Value> {
        &self.values
    }

    pub fn parent(&self) -> Option<SharedKey> {
        self.parent.as_ref().and_then(|weak| weak.upgrade())
    }

    pub fn children(&self) -> Vec<SharedKey> {
        self.children.clone()
    }

    pub fn from(
        name: regashii::KeyName,
        inner: regashii::Key,
        parent: Option<SharedKey>,
    ) -> SharedKey {
        let key = Rc::new(RefCell::new(Key::new(name, inner)));

        // If a parent is provided, add the new key as a child of the parent
        // and store a weak reference to the parent in the new key
        let parent = if let Some(parent) = parent {
            parent.borrow_mut().add_child(key.clone());
            Some(Rc::downgrade(&parent))
        } else {
            None
        };
        key.borrow_mut().parent = parent;

        key
    }
}

pub struct Registry {
    root: SharedKey,
    map: BTreeMap<regashii::KeyName, SharedKey>,
}

impl Registry {
    pub fn root(&self) -> SharedKey {
        Rc::clone(&self.root)
    }

    pub fn get_key(&self, key_name: &regashii::KeyName) -> Option<SharedKey> {
        self.map.get(key_name).cloned()
    }

    pub fn try_from<T: AsRef<std::path::Path>>(file: T) -> Result<Self, regashii::error::Read> {
        let registry = regashii::Registry::deserialize_file(file)?;

        Ok(registry.into())
    }
}

impl From<regashii::Registry> for Registry {
    fn from(registry: regashii::Registry) -> Self {
        let root_name = regashii::KeyName::new("");
        let root: SharedKey = Rc::new(RefCell::new(Key::new(
            root_name.clone(),
            regashii::Key::default(),
        )));
        let mut map = BTreeMap::from([(root_name, root.clone())]);

        for (key_name, _) in registry.keys() {
            let key_segments = key_name.raw().split('\\').collect::<Vec<_>>();
            let mut new_key_name = String::new();
            let mut last_key = Rc::clone(&root);

            for segment in key_segments {
                new_key_name.push_str(segment);
                new_key_name.push('\\');

                let temp_name = regashii::KeyName::new(&new_key_name);

                if let Some(key) = map.get(&temp_name) {
                    last_key = Rc::clone(key);
                    continue;
                }

                let key = registry
                    .keys()
                    .get(&temp_name)
                    .cloned()
                    .unwrap_or(regashii::Key::new());
                let new_key = Key::from(temp_name.clone(), key, Some(last_key));
                map.insert(temp_name, Rc::clone(&new_key));
                last_key = new_key;
            }
        }

        Self { root, map }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_registry() {
        let registry = Registry::try_from("./registries/user.reg");
        assert!(registry.is_ok())
    }

    #[test]
    fn registry_get_key() {
        let registry = Registry::try_from("./registries/user.reg").unwrap();
        let key = registry.get_key(&regashii::KeyName::new("Software\\Wine"));
        assert!(key.is_some());
    }

    #[test]
    fn registry_key_name() {
        let registry = Registry::try_from("./registries/user.reg").unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine"))
            .unwrap();
        assert_eq!(key.borrow().name().raw(), "Software\\Wine");
    }

    #[test]
    fn registry_get_key_none() {
        let registry = Registry::try_from("./registries/user.reg").unwrap();
        let key = registry.get_key(&regashii::KeyName::new("Software\\Wine\\NonExistent"));
        assert!(key.is_none());
    }

    #[test]
    fn registry_root() {
        let registry = Registry::try_from("./registries/user.reg").unwrap();
        let root = registry.root();
        assert_eq!(root.borrow().name().raw(), "");
    }

    #[test]
    fn count_registry_values() {
        let registry = Registry::try_from("./registries/user.reg").unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine\\X11 Driver"))
            .unwrap();
        assert_eq!(key.borrow().values().len(), 1);
    }

    #[test]
    fn get_registry_values() {
        let registry = Registry::try_from("./registries/user.reg").unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine\\X11 Driver"))
            .unwrap();
        assert!(key.borrow().values().iter().nth(0).is_some());
    }

    #[test]
    fn get_registry_values_none() {
        let registry = Registry::try_from("./registries/user.reg").unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine\\X11 Driver"))
            .unwrap();
        assert!(key.borrow().values().iter().nth(999).is_none());
    }

    #[test]
    fn registry_children() {
        let registry = Registry::try_from("./registries/user.reg").unwrap();
        let key = registry.get_key(&regashii::KeyName::new("")).unwrap();
        let children = key.borrow().children();
        assert_eq!(children.len(), 6);
    }
}
