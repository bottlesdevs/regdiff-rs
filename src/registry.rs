use regashii::{KeyName, ValueName};
use std::collections::BTreeMap;

/// The supported registry hives (root keys).
#[derive(Clone, Copy, Debug)]
pub enum Hive {
    /// Represents the HKEY_LOCAL_MACHINE hive.
    LocalMachine,
    /// Represents the HKEY_CURRENT_USER hive.
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

/// Represents a registry value entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Value {
    /// The name of the registry value.
    name: ValueName,
    /// The data associated with the value.
    value: regashii::Value,
}

impl Value {
    /// Constructs a new [Value] with the provided name and data.
    ///
    /// # Arguments
    ///
    /// * `name` - The registry value name.
    /// * `value` - The registry data associated with this value.
    pub fn new(name: ValueName, value: regashii::Value) -> Self {
        Self { name, value }
    }

    /// Returns a reference to the name of the registry value.
    pub fn name(&self) -> &ValueName {
        &self.name
    }

    /// Returns a reference to the registry value's data.
    pub fn value(&self) -> &regashii::Value {
        &self.value
    }

    /// Converts the [Value] into a tuple containing the underlying value name and data.
    pub fn into_regashii_value(self) -> (ValueName, regashii::Value) {
        (self.name, self.value)
    }

    /// Converts the [Value] into a tuple containing the underlying value name and a deleted value.
    pub fn into_deleted_value(self) -> (ValueName, regashii::Value) {
        (self.name, regashii::Value::Delete)
    }
}

/// Represents a registry key, which can contain multiple values.
#[derive(Clone, Debug)]
pub struct Key {
    /// The full registry key name/path.
    name: KeyName,
    /// A map of registry values within the key.
    values: BTreeMap<ValueName, Value>,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.values == other.values
    }
}

impl From<Key> for (KeyName, regashii::Key) {
    fn from(key: Key) -> Self {
        let name = key.name;
        let values = key.values;

        let mut key = regashii::Key::new();

        for (name, value) in values.into_iter() {
            key = key.with(name, value.value)
        }

        (name, key)
    }
}

impl Key {
    /// Constructs a new [Key] instance from a given registry key name and the regashii key.
    ///
    /// This function iterates over each value from the regashii key and wraps them in our own `Value` type.
    ///
    /// # Arguments
    ///
    /// * `name` - The registry key name.
    /// * `key` - The regashii representation of a registry key.
    pub fn new(name: KeyName, key: regashii::Key) -> Self {
        let values = key
            .values()
            .into_iter()
            .map(|(key_name, value)| {
                let new_value = Value::new(key_name.clone(), value.clone());
                (key_name.clone(), new_value)
            })
            .collect();
        Self { name, values }
    }

    /// Returns a reference to the registry key's name.
    pub fn name(&self) -> &KeyName {
        &self.name
    }

    /// Returns a reference to the sorted map of values in the registry key.
    pub fn values(&self) -> &BTreeMap<ValueName, Value> {
        &self.values
    }

    /// Converts the [Key] into a tuple containing the underlying key name and the regashii key.
    pub fn into_regashii_key(self) -> (KeyName, regashii::Key) {
        let name = self.name;
        let values = self.values;

        let mut key = regashii::Key::new();

        for (name, value) in values.into_iter() {
            key = key.with(name, value.value)
        }

        (name, key)
    }

    /// Converts the [Key] into a tuple containing the underlying key name and a deleted key.
    pub fn into_deleted_key(self) -> (KeyName, regashii::Key) {
        (self.name, regashii::Key::deleted())
    }
}

/// Represents the loaded registry data.
///
/// This type is responsible for deserializing registry files and managing a collection
/// of registry keys.
pub struct Registry {
    /// A map of registry keys keyed by their name.
    keys: BTreeMap<KeyName, Key>,
}

impl Registry {
    /// Returns a reference to the entire collection of registry keys.
    pub fn keys(&self) -> &BTreeMap<KeyName, Key> {
        &self.keys
    }

    /// Retrieves a specific registry key by its name.
    ///
    /// # Arguments
    ///
    /// * `name` - A reference to the registry key name.
    ///
    /// # Returns
    ///
    /// `Some(&Key)` if the key exists, or `None` otherwise.
    pub fn key(&self, name: &KeyName) -> Option<&Key> {
        self.keys.get(name)
    }

    /// Attempts to construct a `Registry` from a file.
    ///
    /// This function deserializes a given file path using regashii and then converts the
    /// resulting registry into our custom `Registry` type according to the specified `Hive`.
    ///
    /// # Arguments
    ///
    /// * `file` - A path or a reference to a file path containing registry data.
    /// * `hive` - The registry hive to use for prefixing registry keys.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Registry` or a `regashii::error::Read` error if deserialization fails.
    pub fn try_from<T: AsRef<std::path::Path>>(
        file: T,
        hive: Hive,
    ) -> Result<Self, regashii::error::Read> {
        let registry = regashii::Registry::deserialize_file(file)?;

        Ok(Self::from(registry, hive))
    }

    /// Converts a regashii registry into our custom `Registry` using the provided hive.
    ///
    /// It iterates over all registry keys, prepending the hive to the original key names.
    ///
    /// # Arguments
    ///
    /// * `registry` - The regashii registry instance.
    /// * `hive` - The registry hive that serves as the prefix.
    fn from(registry: regashii::Registry, hive: Hive) -> Self {
        let map = registry
            .keys()
            .into_iter()
            .map(|(name, key)| {
                // Prepend the hive to the existing key name.
                let new_name = KeyName::new(format!("{}\\{}", hive, name.raw()));
                // Create a new Key instance using the updated name.
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
