use regashii::{KeyName, ValueName};
use std::collections::BTreeMap;

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

#[derive(Clone, Debug, PartialEq)]
pub struct Value {
    name: ValueName,
    value: regashii::Value,
}

impl Into<regashii::Key> for Key {
    fn into(self) -> regashii::Key {
        let mut key = if self.deleted {
            regashii::Key::deleted()
        } else {
            regashii::Key::new()
        };

        for (name, value) in self.values.into_iter() {
            key = key.with(name, value.value)
        }

        key
    }
}

impl Value {
    pub fn new(name: ValueName, value: regashii::Value) -> Self {
        Self { name, value }
    }

    pub fn name(&self) -> &ValueName {
        &self.name
    }

    pub fn value(&self) -> &regashii::Value {
        &self.value
    }
}

#[derive(Clone, Debug)]
pub struct Key {
    name: KeyName,
    values: BTreeMap<ValueName, Value>,
    deleted: bool,
}

impl Key {
    pub fn new(name: KeyName, key: regashii::Key) -> Self {
        let values = key
            .values()
            .into_iter()
            .map(|(key_name, value)| {
                let new_value = Value::new(key_name.clone(), value.clone());
                (key_name.clone(), new_value)
            })
            .collect();
        Self {
            name,
            values,
            deleted: false,
        }
    }

    pub fn name(&self) -> &KeyName {
        &self.name
    }

    pub fn values(&self) -> &BTreeMap<ValueName, Value> {
        &self.values
    }

    pub fn deleted(name: KeyName) -> Self {
        Self {
            name,
            values: BTreeMap::new(),
            deleted: true,
        }
    }
}

pub struct Registry {
    keys: BTreeMap<KeyName, Key>,
}

impl Registry {
    pub fn keys(&self) -> &BTreeMap<KeyName, Key> {
        &self.keys
    }

    pub fn key(&self, name: &KeyName) -> Option<&Key> {
        self.keys.get(name)
    }

    pub fn try_from<T: AsRef<std::path::Path>>(
        file: T,
        hive: Hive,
    ) -> Result<Self, regashii::error::Read> {
        let registry = regashii::Registry::deserialize_file(file)?;

        Ok(Self::from(registry, hive))
    }

    fn from(registry: regashii::Registry, hive: Hive) -> Self {
        let map = registry
            .keys()
            .into_iter()
            .map(|(name, key)| {
                let new_name = KeyName::new(format!("{}\\{}", hive, name.raw()));
                let new_key = Key::new(new_name.clone(), key.clone());
                (name.clone(), new_key)
            })
            .collect();

        Self { keys: map }
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
    fn test_get_existing_registry_key() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry.key(&regashii::KeyName::new("Software\\Wine\\Fonts"));
        assert!(key.is_some());
    }

    #[test]
    fn test_registry_key_has_correct_name() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .key(&regashii::KeyName::new("Software\\Wine\\Fonts"))
            .unwrap();
        assert_eq!(key.name().raw(), "HKEY_CURRENT_USER\\Software\\Wine\\Fonts");
    }

    #[test]
    fn test_get_nonexistent_registry_key_returns_none() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry.key(&regashii::KeyName::new("Software\\Wine\\NonExistent"));
        assert!(key.is_none());
    }

    #[test]
    fn test_registry_key_value_count_is_correct() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .key(&regashii::KeyName::new("Software\\Wine\\X11 Driver"))
            .unwrap();
        assert_eq!(key.values().len(), 1);
    }

    #[test]
    fn test_registry_key_contains_expected_values() {
        let registry = Registry::try_from("./registries/user.reg", Hive::CurrentUser).unwrap();
        let key = registry
            .key(&regashii::KeyName::new(
                "Software\\Wine\\Fonts\\Replacements",
            ))
            .unwrap();

        let value = key
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
            .key(&regashii::KeyName::new("Software\\Wine\\X11 Driver"))
            .unwrap();
        assert!(key.values().iter().nth(999).is_none());
    }
}
