use regdiff_rs::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open a registry file, this operation might take a while because it needs to generate the tree for the registry
    let o_reg = Registry::try_from("./registries/system.old.reg")?;
    let n_reg = Registry::try_from("./registries/system.new.reg")?;

    let mut patch = regashii::Registry::new(regashii::Format::Wine2);
    let diff = Diff::diff(Some(&o_reg.root()), Some(&n_reg.root()));

    for op in diff {
        patch = match op {
            Operation::Add { name, data } => patch.with(name, data),
            Operation::Delete { name } => patch.with(name, regashii::Key::deleted()),
            Operation::Update { name, old, new } => {
                patch = patch.with(name.clone(), regashii::Key::deleted());
                patch.with(name, new)
            }
            _ => patch,
        };
    }
    patch.serialize_file("patch.reg")?;
    Ok(())
}
