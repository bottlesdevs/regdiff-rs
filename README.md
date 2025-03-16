# RegDiff – A Windows Registry Diff and Patch Utility

## Overview
RegDiff is a Rust library designed to calculate differences between two Windows Registry files. It inspects two registry snapshots, computes the modifications (additions, deletions, and updates) in registry keys and their respective values, and outputs a patch file representing the diff. Under the hood, RegDiff leverages the [regashii](https://crates.io/crates/regashii) crate for registry serialization and deserialization.

## Usage
Add `regdiff` as a dependency in `Cargo.toml`:

```toml
[dependencies]
regdiff-rs = "*"
```

RegDiff provides a `Diff` trait implemented for registry keys and values. You can calculate a diff between two registries as follows:

```rust
use regdiff_rs::prelude::{Diff, Hive, Registry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the old and new registry snapshots. Hive needs to be manually specified as Wine registry files do not contain hive information.
    let o_reg = Registry::try_from("./registries/old.reg", Hive::LocalMachine)?;
    let n_reg = Registry::try_from("./registries/new.reg", Hive::LocalMachine)?;

    // Calculate the difference between registries.
    let diff = Registry::diff(&o_reg, &n_reg);

    // Serialize the diff patch to a file.
    diff.serialize_file("patch.reg")?;

    Ok(())
}
```

### Using the Example Executable
An example executable is available under the `examples` directory. To run the example:

```bash
cargo run --example diff
```

## Contributing
Contributions, issues, and feature requests are welcome! Feel free to check the issue tracker or submit a pull request on GitHub.

## License
This project is licensed under the GPLv3 License – see the [LICENSE](LICENSE) file for details.
