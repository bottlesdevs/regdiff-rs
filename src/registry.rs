use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

#[derive(Clone, Debug)]
pub enum Hive {
    LocalMachine,
    CurrentUser,
}

impl std::fmt::Display for Hive {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Hive::LocalMachine => "HKEY_LOCAL_MACHINE",
                Hive::CurrentUser => "HKEY_CURRENT_USER",
            }
        )
    }
}

impl From<Hive> for regashii::KeyName {
    fn from(hive: Hive) -> Self {
        regashii::KeyName::new(&hive.to_string())
    }
}

pub type SharedKey = Rc<RefCell<Key>>;

#[derive(Clone, Debug, PartialEq)]
pub struct Value {
    name: regashii::ValueName,
    value: regashii::Value,
}

impl Value {
    pub fn from(name: regashii::ValueName, value: regashii::Value) -> Self {
        Self { name, value }
    }

    pub fn name(&self) -> &regashii::ValueName {
        &self.name
    }

    pub fn value(&self) -> &regashii::Value {
        &self.value
    }
}

#[derive(Debug)]
pub struct Key {
    name: String,
    parent: Option<Weak<RefCell<Key>>>,
    children: Vec<SharedKey>,
    values: BTreeMap<regashii::ValueName, Value>,
    inner: regashii::Key,
}

impl Key {
    pub fn new(name: &str, inner: regashii::Key) -> Self {
        let values = inner
            .values()
            .iter()
            .map(|(name, value)| (name.clone(), Value::from(name.clone(), value.clone())))
            .collect();

        Self {
            name: name.to_string(),
            parent: None,
            values,
            children: Vec::new(),
            inner,
        }
    }

    pub fn add_child(&mut self, child: SharedKey) {
        self.children.push(child);
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> regashii::KeyName {
        let mut segments = vec![self.name().to_string()];
        let mut parent = self.parent();

        while let Some(key) = parent {
            let name = key.borrow().name().to_string();
            segments.push(name);
            parent = key.borrow().parent();
        }

        segments.reverse();

        regashii::KeyName::new(&segments.join("\\"))
    }

    pub fn values(&self) -> &BTreeMap<regashii::ValueName, Value> {
        &self.values
    }

    pub fn parent(&self) -> Option<SharedKey> {
        self.parent.as_ref().and_then(|weak| weak.upgrade())
    }

    pub fn children(&self) -> Vec<SharedKey> {
        self.children.clone()
    }

    pub fn from(name: &str, inner: regashii::Key, parent: Option<SharedKey>) -> SharedKey {
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

    pub fn inner(&self) -> &regashii::Key {
        &self.inner
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

    pub fn try_from<T: AsRef<std::path::Path>>(
        file: T,
        hive: Hive,
    ) -> Result<Self, regashii::error::Read> {
        let registry = regashii::Registry::deserialize_file(file)?;

        Ok(Self::from(registry, hive))
    }

    fn from(registry: regashii::Registry, hive: Hive) -> Self {
        let root_name: regashii::KeyName = hive.into();
        let root: SharedKey = Rc::new(RefCell::new(Key::new(
            root_name.raw(),
            regashii::Key::default(),
        )));
        let mut map = BTreeMap::from([(regashii::KeyName::new(""), root.clone())]);

        for (key_name, _) in registry.keys() {
            let key_segments = key_name.raw().split('\\').collect::<Vec<_>>();
            let mut key_path = String::new();
            let mut last_key = Rc::clone(&root);

            for segment in key_segments {
                key_path.push_str(segment);
                key_path.push('\\');

                let temp_keyname = regashii::KeyName::new(key_path.clone());
                if let Some(key) = map.get(&temp_keyname) {
                    last_key = Rc::clone(key);
                    continue;
                }

                let key = registry
                    .keys()
                    .get(&temp_keyname)
                    .cloned()
                    .unwrap_or(regashii::Key::new());
                let new_key = Key::from(segment, key, Some(last_key));
                map.insert(temp_keyname, Rc::clone(&new_key));
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
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser);
        assert!(registry.is_ok())
    }

    #[test]
    fn registry_get_key() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry.get_key(&regashii::KeyName::new("Software\\Wine"));
        assert!(key.is_some());
    }

    #[test]
    fn registry_key_name() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine"))
            .unwrap();
        assert_eq!(key.borrow().name(), "Wine");
    }

    #[test]
    fn registry_get_key_none() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry.get_key(&regashii::KeyName::new("Software\\Wine\\NonExistent"));
        assert!(key.is_none());
    }

    #[test]
    fn registry_get_key_path() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine"))
            .unwrap();
        assert_eq!(
            key.borrow().path().raw(),
            "HKEY_CURRENT_USER\\Software\\Wine"
        );
    }

    #[test]
    fn registry_root() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let root = registry.root();
        assert_eq!(root.borrow().name(), "HKEY_CURRENT_USER");
    }

    #[test]
    fn count_registry_values() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine\\X11 Driver"))
            .unwrap();
        assert_eq!(key.borrow().values().len(), 1);
    }

    #[test]
    fn get_registry_values() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine\\X11 Driver"))
            .unwrap();
        assert!(key.borrow().values().iter().nth(0).is_some());
    }

    #[test]
    fn get_registry_values_none() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine\\X11 Driver"))
            .unwrap();
        assert!(key.borrow().values().iter().nth(999).is_none());
    }

    #[test]
    fn registry_children() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry.get_key(&regashii::KeyName::new("")).unwrap();
        let children = key.borrow().children();
        assert_eq!(children.len(), 6);
    }
}
