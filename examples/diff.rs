use regdiff_rs::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open a registry file, this operation might take a while because it needs to generate the tree for the registry
    let o_reg = Registry::try_from("./registries/old.reg", Hive::LocalMachine)?;
    let n_reg = Registry::try_from("./registries/new.reg", Hive::LocalMachine)?;

    let diff = Diff::diff(Some(&o_reg), Some(&n_reg));

    diff.serialize_file("patch.reg")?;
    Ok(())
}
