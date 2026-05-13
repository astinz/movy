use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use color_eyre::eyre::eyre;
use itertools::Itertools;
use log::{debug, trace};
use move_binary_format::CompiledModule;
use move_core_types::account_address::AccountAddress;
use movy_types::{
    abi::{MOVY_INIT, MOVY_ORACLE, MovePackageAbi},
    error::MovyError,
    input::MoveAddress,
};
use serde::{Deserialize, Serialize};
use sui_move_build::{BuildConfig, CompiledPackage};
use sui_types::base_types::ObjectID;

pub fn build_package_resolved(
    folder: &Path,
    test_mode: bool,
) -> Result<CompiledPackage, MovyError> {
    let mut cfg = BuildConfig::new_for_testing();
    cfg.config.test_mode = test_mode;
    cfg.run_bytecode_verifier = false;
    cfg.print_diags_to_stderr = false;
    trace!("Build config is {:?}", &cfg.config);
    Ok(cfg.build(folder)?)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiCompiledPackage {
    pub package_id: ObjectID,
    pub package_name: String,
    pub package_names: Vec<String>,
    #[serde(with = "compiled_modules_serde")]
    modules: Vec<CompiledModule>,
    dependencies: Vec<ObjectID>,
    published_dependencies: Vec<ObjectID>,
}

mod compiled_modules_serde {
    use move_binary_format::{
        CompiledModule, binary_config::BinaryConfig, file_format_common::VERSION_MAX,
    };
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as DeError};

    pub fn serialize<S>(modules: &[CompiledModule], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialized_modules = Vec::with_capacity(modules.len());
        for module in modules {
            let mut bytes = Vec::new();
            module
                .serialize_with_version(VERSION_MAX, &mut bytes)
                .map_err(serde::ser::Error::custom)?;
            serialized_modules.push(bytes);
        }
        serialized_modules.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<CompiledModule>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<Vec<u8>>::deserialize(deserializer)?
            .into_iter()
            .map(|bytes| {
                CompiledModule::deserialize_with_config(&bytes, &BinaryConfig::new_unpublishable())
                    .map_err(D::Error::custom)
            })
            .collect()
    }
}

impl SuiCompiledPackage {
    pub fn all_modules_iter(&self) -> impl Iterator<Item = &CompiledModule> {
        self.modules.iter()
    }
    pub fn dependencies(&self) -> &[ObjectID] {
        &self.dependencies
    }
    pub fn into_deployment(self) -> (Vec<CompiledModule>, Vec<ObjectID>) {
        (self.modules.into_iter().collect(), self.dependencies)
    }
    pub fn abi(&self) -> Result<MovePackageAbi, MovyError> {
        MovePackageAbi::from_sui_id_and_modules(self.package_id, self.all_modules_iter())
    }
}

impl SuiCompiledPackage {
    pub fn test_modules(&self) -> Vec<&CompiledModule> {
        self.modules
            .iter()
            .filter(|v| Self::contains_unit_test(v))
            .collect()
    }
    fn contains_unit_test(module: &CompiledModule) -> bool {
        for fcall in module.function_handles() {
            let md = module.module_handle_at(fcall.module);
            let fname = module.identifier_at(fcall.name);
            let maddress = module.address_identifier_at(md.address);
            let mname = module.identifier_at(md.name);
            if MoveAddress::from(*maddress) == MoveAddress::one()
                && mname.as_str() == "unit_test"
                && fname.as_str() == "poison"
            {
                return true;
            }
        }
        false
    }
    fn contains_movy(modlue: &CompiledModule) -> bool {
        for fdef in modlue.function_defs() {
            let func = modlue.function_handle_at(fdef.function);
            let fname = modlue.identifier_at(func.name).to_string();
            if fname == MOVY_INIT || fname.starts_with(MOVY_ORACLE) {
                return true;
            }
        }

        false
    }

    fn mock_module(md: &CompiledModule) -> CompiledModule {
        let mut md = md.clone();
        let address: MoveAddress = (*md.address()).into();
        let mname = md.name().to_string();
        if !md.publishable {
            log::debug!("Mock module publishable {}::{}", address, mname);
            md.publishable = true;
        };
        md
    }

    pub fn movy_mock(&self) -> Result<Self, MovyError> {
        let mut new_package = Self {
            package_id: self.package_id,
            package_name: self.package_name.clone(),
            package_names: self.package_names.clone(),
            modules: vec![],
            dependencies: vec![],
            published_dependencies: self.published_dependencies.clone(),
        };

        let deps: BTreeSet<ObjectID> = self.dependencies.clone().into_iter().collect();
        for md in self.modules.iter() {
            let md = Self::mock_module(md);
            // deps.extend(
            //     md.immediate_dependencies()
            //         .into_iter()
            //         .map(|v| ObjectID::from(*v.address()))
            //         .filter(|v| v != &self.package_id),
            // );
            new_package.modules.push(md);
        }
        new_package.dependencies = deps.into_iter().collect();
        Ok(new_package)
    }

    // This function builds all sui packages at a given folder, packaging all the
    // unpublished dependencies together.
    pub fn build_all_unpublished_from_folder(
        folder: &Path,
        test_mode: bool,
    ) -> Result<SuiCompiledPackage, MovyError> {
        let artifacts = build_package_resolved(folder, test_mode)?;
        debug!("published: {:?}", artifacts.dependency_ids.published);

        let root_address = artifacts.published_at.unwrap_or(ObjectID::ZERO);
        debug!("Root address is {}", root_address);
        let package_name = artifacts
            .package
            .compiled_package_info
            .package_name
            .to_string();
        let mut package_names = BTreeSet::new();
        package_names.insert(package_name.clone());
        for (dep_name, unit) in artifacts.package.deps_compiled_units.iter() {
            let module_addr: MoveAddress = (*unit.unit.module.self_id().address()).into();
            if ObjectID::from(module_addr) == root_address {
                package_names.insert(dep_name.to_string());
            }
        }
        let package_names = package_names.into_iter().collect::<Vec<_>>();
        let mut modules = artifacts
            .package
            .all_compiled_units()
            .filter(|m| {
                debug!("Compiled module address: {}", m.address.into_inner());
                root_address == m.address.into_inner().into()
            })
            .map(|m| m.module.clone())
            .collect::<Vec<_>>();
        if modules.len() == 0 {
            return Err(eyre!(
                "Compiling {} yields 0 modules for root {}",
                folder.display(),
                root_address
            )
            .into());
        }
        debug!("Package {} has {} modules", root_address, modules.len());
        let mut dep_packages = artifacts
            .dependency_ids
            .published
            .iter()
            .map(|(package, id)| (package.to_string(), *id))
            .collect::<BTreeMap<_, _>>();
        let mut published_dependencies = artifacts
            .dependency_ids
            .published
            .values()
            .cloned()
            .collect::<Vec<_>>();

        if let Some(environment_name) = infer_environment_from_move_lock(folder)? {
            let remaps = environment_dependency_remaps(&artifacts, &environment_name)?;
            if !remaps.storage_by_package.is_empty() || !remaps.address_by_id.is_empty() {
                debug!(
                    "Applying {} package-id and {} address remaps for {} environment",
                    remaps.storage_by_package.len(),
                    remaps.address_by_id.len(),
                    environment_name
                );
                apply_dependency_storage_remaps(
                    &mut dep_packages,
                    &mut published_dependencies,
                    &remaps.storage_by_package,
                );
                remap_module_addresses(&mut modules, &remaps.address_by_id);
            }
        }

        let deps = dep_packages
            .values()
            .copied()
            .collect::<BTreeSet<ObjectID>>();
        debug!(
            "Package {} uses published dependencies {}",
            root_address,
            deps.iter().map(|t| t.to_string()).join(",")
        );
        Ok(SuiCompiledPackage {
            package_id: (*root_address).into(),
            package_name,
            package_names,
            modules,
            dependencies: deps.into_iter().collect(),
            published_dependencies,
        })
    }

    pub fn build_quick(package: &str, module: &str, content: &str) -> Result<Self, MovyError> {
        let dir = tempfile::TempDir::new()?;

        let toml = format!(
            r#"[package]
    name = "{}"
    edition = "2024.beta"

    [dependencies]
    [addresses]
    {} = "0x0"
    [dev-dependencies]
    [dev-addresses]
    "#,
            package, package
        );
        let mut fp = std::fs::File::create(dir.path().join("Move.toml"))?;
        fp.write_all(toml.as_bytes())?;

        std::fs::create_dir_all(dir.path().join("sources"))?;

        let mut fp = std::fs::File::create(dir.path().join(format!("sources/{}.move", module)))?;
        fp.write_all(content.as_bytes())?;

        Self::build_all_unpublished_from_folder(dir.path(), false)
    }
}

#[derive(Debug, Default)]
struct EnvironmentDependencyRemaps {
    storage_by_package: BTreeMap<String, ObjectID>,
    address_by_id: BTreeMap<ObjectID, ObjectID>,
}

fn infer_environment_from_move_lock(folder: &Path) -> Result<Option<String>, MovyError> {
    let lock_path = folder.join("Move.lock");
    if !lock_path.is_file() {
        return Ok(None);
    }

    let lock: toml::Value = toml::from_str(&fs::read_to_string(&lock_path)?)?;
    let Some(pinned) = lock.get("pinned").and_then(toml::Value::as_table) else {
        return Ok(None);
    };

    for packages in pinned.values().filter_map(toml::Value::as_table) {
        for package in packages.values().filter_map(toml::Value::as_table) {
            let source = package.get("source").and_then(toml::Value::as_table);
            let is_root = source
                .and_then(|source| source.get("root"))
                .and_then(toml::Value::as_bool)
                .unwrap_or(false);
            if is_root {
                return Ok(package
                    .get("use_environment")
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned));
            }
        }
    }

    if pinned.len() == 1 {
        return Ok(pinned.keys().next().map(ToOwned::to_owned));
    }

    Ok(None)
}

fn environment_dependency_remaps(
    artifacts: &CompiledPackage,
    environment_name: &str,
) -> Result<EnvironmentDependencyRemaps, MovyError> {
    let mut remaps = EnvironmentDependencyRemaps::default();
    let mut package_roots = BTreeSet::<(String, PathBuf)>::new();

    for (dep_name, unit) in artifacts.package.deps_compiled_units.iter() {
        if let Some(package_root) = package_root_for_source(&unit.source_path) {
            package_roots.insert((dep_name.to_string(), package_root));
        }
    }

    for (dep_name, package_root) in package_roots {
        let default_manifest_path = package_root.join("Move.toml");
        let environment_manifest_path = package_root.join(format!("Move.{environment_name}.toml"));
        if !environment_manifest_path.is_file() {
            continue;
        }

        let Some(default_manifest) = read_manifest(&default_manifest_path)? else {
            continue;
        };
        let Some(environment_manifest) = read_manifest(&environment_manifest_path)? else {
            continue;
        };

        let default_published = manifest_published_at(&default_manifest, &default_manifest_path)?;
        let environment_published =
            manifest_published_at(&environment_manifest, &environment_manifest_path)?;

        if let Some(environment_published) = environment_published {
            remaps
                .storage_by_package
                .insert(dep_name.clone(), environment_published);
            if let Some(package_name) = manifest_package_name(&environment_manifest) {
                remaps
                    .storage_by_package
                    .insert(package_name.to_owned(), environment_published);
            }
        }

        if let (Some(default_published), Some(environment_published)) =
            (default_published, environment_published)
            && default_published != environment_published
        {
            debug!(
                "Remapping dependency package {} storage {} -> {} from {}",
                dep_name,
                default_published,
                environment_published,
                environment_manifest_path.display()
            );
            remaps
                .address_by_id
                .insert(default_published, environment_published);
        }

        let default_addresses = manifest_addresses(&default_manifest, &default_manifest_path)?;
        let environment_addresses =
            manifest_addresses(&environment_manifest, &environment_manifest_path)?;
        for (name, default_address) in default_addresses {
            let Some(environment_address) = environment_addresses.get(&name).copied() else {
                continue;
            };
            if default_address != environment_address {
                debug!(
                    "Remapping dependency address {}={} -> {} from {}",
                    name,
                    default_address,
                    environment_address,
                    environment_manifest_path.display()
                );
                remaps
                    .address_by_id
                    .insert(default_address, environment_address);
            }
        }
    }

    Ok(remaps)
}

fn package_root_for_source(source_path: &Path) -> Option<PathBuf> {
    let mut cursor = if source_path.is_file() {
        source_path.parent()?
    } else {
        source_path
    };

    loop {
        if cursor.join("Move.toml").is_file() {
            return Some(cursor.to_path_buf());
        }
        cursor = cursor.parent()?;
    }
}

fn read_manifest(path: &Path) -> Result<Option<toml::Value>, MovyError> {
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(toml::from_str(&fs::read_to_string(path)?)?))
}

fn manifest_package_name(manifest: &toml::Value) -> Option<&str> {
    manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
}

fn manifest_published_at(
    manifest: &toml::Value,
    manifest_path: &Path,
) -> Result<Option<ObjectID>, MovyError> {
    manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get("published-at"))
        .and_then(toml::Value::as_str)
        .map(|address| parse_manifest_object_id(address, manifest_path))
        .transpose()
}

fn manifest_addresses(
    manifest: &toml::Value,
    manifest_path: &Path,
) -> Result<BTreeMap<String, ObjectID>, MovyError> {
    let Some(addresses) = manifest.get("addresses").and_then(toml::Value::as_table) else {
        return Ok(BTreeMap::new());
    };

    addresses
        .iter()
        .filter_map(|(name, value)| value.as_str().map(|address| (name, address)))
        .map(|(name, address)| {
            parse_manifest_object_id(address, manifest_path).map(|parsed| (name.to_owned(), parsed))
        })
        .collect()
}

fn parse_manifest_object_id(address: &str, manifest_path: &Path) -> Result<ObjectID, MovyError> {
    ObjectID::from_str(address).map_err(|error| {
        eyre!(
            "Invalid object id {} in {}: {}",
            address,
            manifest_path.display(),
            error
        )
        .into()
    })
}

fn apply_dependency_storage_remaps(
    dep_packages: &mut BTreeMap<String, ObjectID>,
    published_dependencies: &mut Vec<ObjectID>,
    storage_by_package: &BTreeMap<String, ObjectID>,
) {
    let previous_by_package = dep_packages.clone();

    for (package, storage_id) in dep_packages.iter_mut() {
        if let Some(environment_storage_id) = storage_by_package.get(package).copied() {
            *storage_id = environment_storage_id;
        }
    }

    let previous_to_environment = previous_by_package
        .iter()
        .filter_map(|(package, previous)| {
            storage_by_package
                .get(package)
                .copied()
                .map(|environment| (*previous, environment))
        })
        .collect::<BTreeMap<_, _>>();

    for dependency in published_dependencies.iter_mut() {
        if let Some(environment_storage_id) = previous_to_environment.get(dependency).copied() {
            *dependency = environment_storage_id;
        }
    }
    published_dependencies.sort();
    published_dependencies.dedup();
}

fn remap_module_addresses(
    modules: &mut [CompiledModule],
    address_by_id: &BTreeMap<ObjectID, ObjectID>,
) {
    for module in modules {
        for address in module.address_identifiers.iter_mut() {
            if let Some(environment_address) = address_by_id.get(&ObjectID::from(*address)) {
                *address = AccountAddress::from(*environment_address);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::{HashMap, HashSet},
        path::PathBuf,
    };

    use crate::compile::{SuiCompiledPackage, build_package_resolved};

    #[test]
    fn test_build_simple() {
        let out = SuiCompiledPackage::build_quick(
            "hello",
            "hello",
            r#"module hello::hello;
public struct Test<T: drop, Z: drop> {
    t: T,
    k: Z
}
public fun new<T: drop, V: drop>(ctx: &mut TxContext, t2: &Test<T, V>, t: &Test<u64, V>) {
}
"#,
        )
        .unwrap();
        let abi = out.abi().unwrap();
        dbg!(&abi);
    }
}
