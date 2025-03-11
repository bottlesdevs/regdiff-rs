use regashii::Value;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

pub type SharedKey = Rc<RefCell<Key>>;

#[derive(Debug)]
pub struct Key {
    name: regashii::KeyName,
    inner: regashii::Key,
    parent: Option<Weak<RefCell<Key>>>,
    children: Vec<SharedKey>,
}

impl Key {
    pub fn new(name: regashii::KeyName, inner: regashii::Key) -> Self {
        Self {
            name,
            inner,
            parent: None,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: SharedKey) {
        self.children.push(child);
    }

    pub fn name(&self) -> &regashii::KeyName {
        &self.name
    }

    pub fn values(&self) -> Vec<&Value> {
        self.inner.values().iter().map(|(_, value)| value).collect()
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
