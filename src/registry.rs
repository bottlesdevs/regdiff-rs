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
    key_name: regashii::KeyName,
}

impl Value {
    pub fn from(
        name: regashii::ValueName,
        value: regashii::Value,
        key_name: regashii::KeyName,
    ) -> Self {
        Self {
            name,
            value,
            key_name,
        }
    }

    pub fn name(&self) -> &regashii::ValueName {
        &self.name
    }

    pub fn value(&self) -> &regashii::Value {
        &self.value
    }

    pub fn key_name(&self) -> &regashii::KeyName {
        &self.key_name
    }
}

#[derive(Debug)]
pub struct Key {
    name: String,
    path: regashii::KeyName,
    parent: Option<Weak<RefCell<Key>>>,
    children: Vec<SharedKey>,
    values: BTreeMap<regashii::ValueName, Value>,
}

impl Key {
    pub fn new(
        name: &str,
        values: &BTreeMap<regashii::ValueName, regashii::Value>,
        parent: Option<SharedKey>,
    ) -> SharedKey {
        let path = Self::generate_path(name, parent.clone());
        let values = values
            .iter()
            .map(|(name, value)| {
                (
                    name.clone(),
                    Value::from(name.clone(), value.clone(), path.clone()),
                )
            })
            .collect();

        let key = Rc::new(RefCell::new(Self {
            name: name.to_string(),
            path,
            parent: None,
            values,
            children: Vec::new(),
        }));

        let parent = if let Some(parent) = parent {
            parent.borrow_mut().add_child(key.clone());
            Some(Rc::downgrade(&parent))
        } else {
            None
        };
        key.borrow_mut().parent = parent;

        key
    }

    fn generate_path(name: &str, mut parent: Option<SharedKey>) -> regashii::KeyName {
        let mut segments = vec![name.to_string()];
        while let Some(key) = parent {
            let name = key.borrow().name().to_string();
            segments.push(name);
            parent = key.borrow().parent();
        }

        segments.reverse();
        regashii::KeyName::new(&segments.join("\\"))
    }

    pub fn add_child(&mut self, child: SharedKey) {
        self.children.push(child);
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &regashii::KeyName {
        &self.path
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

    pub fn inner(&self) -> regashii::Key {
        let mut key = regashii::Key::new();

        for value in self.values.values() {
            key = key.with(value.name().clone(), value.value().clone());
        }

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

    pub fn try_from<T: AsRef<std::path::Path>>(
        file: T,
        hive: Hive,
    ) -> Result<Self, regashii::error::Read> {
        let registry = regashii::Registry::deserialize_file(file)?;

        Ok(Self::from(registry, hive))
    }

    fn from(registry: regashii::Registry, hive: Hive) -> Self {
        let root_path: regashii::KeyName = hive.into();
        let root: SharedKey = Key::new(root_path.raw(), &BTreeMap::new(), None);
        let mut map = BTreeMap::from([(regashii::KeyName::new(""), root.clone())]);

        for (key_name, _) in registry.keys() {
            Self::create_key(key_name, registry.keys(), &mut map);
        }

        Self { root, map }
    }

    fn create_key(
        path: &regashii::KeyName,
        keys: &BTreeMap<regashii::KeyName, regashii::Key>,
        map: &mut BTreeMap<regashii::KeyName, SharedKey>,
    ) -> SharedKey {
        if let Some(key) = map.get(path) {
            return key.clone();
        }

        let inner = keys.get(path).cloned().unwrap_or(regashii::Key::new());
        let mut segments: Vec<&str> = path.raw().split('\\').collect();
        let name = segments.pop().unwrap();
        let parent_path = regashii::KeyName::new(segments.join("\\"));

        let parent = map
            .get(&parent_path)
            .cloned()
            .unwrap_or_else(|| Self::create_key(&parent_path, keys, map));

        let key = Key::new(name, inner.values(), Some(parent));
        map.insert(path.clone(), key.clone());
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_registry_success() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser);
        assert!(registry.is_ok())
    }

    #[test]
    fn test_registry_get_root_key() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let root = registry.root();
        assert_eq!(root.borrow().name(), "HKEY_CURRENT_USER");
        assert_eq!(root.borrow().path().raw(), "HKEY_CURRENT_USER");
    }

    #[test]
    fn test_get_existing_registry_key() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry.get_key(&regashii::KeyName::new("Software\\Wine"));
        assert!(key.is_some());
    }

    #[test]
    fn test_registry_key_has_correct_name() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine"))
            .unwrap();
        assert_eq!(key.borrow().name(), "Wine");
    }

    #[test]
    fn test_registry_key_path_is_correct() {
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
    fn test_get_nonexistent_registry_key_returns_none() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry.get_key(&regashii::KeyName::new("Software\\Wine\\NonExistent"));
        assert!(key.is_none());
    }

    #[test]
    fn test_registry_key_value_count_is_correct() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine\\X11 Driver"))
            .unwrap();
        assert_eq!(key.borrow().values().len(), 1);
    }

    #[test]
    fn test_registry_key_contains_expected_values() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new(
                "Software\\Wine\\Fonts\\Replacements",
            ))
            .unwrap();

        let value = key
            .borrow()
            .values()
            .get(&regashii::ValueName::named("Arial Unicode MS"))
            .cloned()
            .unwrap();

        assert_eq!(
            value.value,
            regashii::Value::Sz("Droid Sans Fallback".to_string())
        );
    }

    #[test]
    fn test_registry_key_value_index_out_of_range_returns_none() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .get_key(&regashii::KeyName::new("Software\\Wine\\X11 Driver"))
            .unwrap();
        assert!(key.borrow().values().iter().nth(999).is_none());
    }

    #[test]
    fn test_root_key_children_count_is_correct() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry.get_key(&regashii::KeyName::new("")).unwrap();
        let children = key.borrow().children();
        assert_eq!(children.len(), 6);
    }
}
