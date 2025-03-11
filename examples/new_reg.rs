use regdiff_rs::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open a registry file, this operation might take a while because it needs to generate the tree for the registry
    let registry = Registry::try_from("./registries/user.reg")?;

    let key = registry.get_key(&KeyName::new(""));
    if let Some(key) = key {
        println!("Keys:");
        for child in key.borrow().children() {
            println!("  {:?}", child.borrow().name());
        }

        println!("Values:");
        for value in key.borrow().values() {
            println!("  {:?}", value);
        }
    }

    Ok(())
}
