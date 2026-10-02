//! Read access to one completed gameplay equipment export generation.
//!
//! **Role:** Opens `assets/equipment/gameplay/generations/<id>/export/`, checks that the
//! generation is complete and names the requested id, and hands out resource documents by GUID
//! after verifying each file's bytes against the export manifest's SHA-256.
//!
//! **Position:** The first reader of `cargo xtask ballistics trim-export`; the catalog and table
//! readers query resources through it and never open export files themselves.
//!
//! **Signals & state:** [`GameplayExport`] holds the parsed generation header, the resource index
//! and the manifest hashes; each [`ExportedResource`] owns its parsed document.
//!
//! **Invariants:** A missing directory or file, an unfinished generation, a generation id that
//! differs from the folder, a GUID absent from the index and a file whose SHA-256 differs from the
//! manifest are all errors: the trim never reads an unverified byte.
use anyhow::{Context, Result, bail};
use content_digest::sha256_hex;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Reads and parses one JSON file, naming the file in every error.
pub(crate) fn read_json_file(path: &Path) -> Result<(Vec<u8>, Value)> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let value =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))?;
    Ok((bytes, value))
}

/// One indexed resource file of the export.
struct IndexedResource {
    resource_name: String,
    resource_file: String,
}

/// An opened, completed gameplay export generation.
pub(crate) struct GameplayExport {
    directory: PathBuf,
    generation_id: String,
    game_build: String,
    index: BTreeMap<String, IndexedResource>,
    manifest_sha256: BTreeMap<String, String>,
}

impl GameplayExport {
    /// Opens the export under `directory` and checks it is the completed generation
    /// `generation_id`.
    pub(crate) fn open(directory: &Path, generation_id: &str) -> Result<Self> {
        if !directory.is_dir() {
            bail!(
                "gameplay export {} is missing: run the equipment export for generation \
                 {generation_id} first",
                directory.display()
            );
        }
        let (_, generation) = read_json_file(&directory.join("generation.json"))?;
        if generation["generation_id"] != generation_id {
            bail!(
                "generation.json names generation {} but the trim asked for {generation_id}",
                generation["generation_id"]
            );
        }
        if generation["status"] != "completed" {
            bail!(
                "generation {generation_id} has status {} and is not complete",
                generation["status"]
            );
        }
        let Some(game_build) = generation["environment"]["game_build"]["value"].as_str() else {
            bail!("generation {generation_id} records no game build");
        };
        let (_, manifest) = read_json_file(&directory.join("manifest.json"))?;
        let manifest_sha256 = manifest["files"]
            .as_object()
            .context("manifest.json has no files table")?
            .iter()
            .filter_map(|(file, entry)| Some((file.clone(), entry["sha256"].as_str()?.to_owned())))
            .collect();
        let (_, resource_index) = read_json_file(&directory.join("resource_index.json"))?;
        let mut index = BTreeMap::new();
        for entry in resource_index["resources"].as_array().into_iter().flatten() {
            let (Some(id), Some(name), Some(file)) = (
                entry["resource_id"].as_str(),
                entry["resource_name"].as_str(),
                entry["resource_file"].as_str(),
            ) else {
                continue;
            };
            let guid = id.strip_prefix("guid:").unwrap_or(id).to_owned();
            index.insert(
                guid,
                IndexedResource {
                    resource_name: name.to_owned(),
                    resource_file: file.to_owned(),
                },
            );
        }
        Ok(Self {
            directory: directory.to_path_buf(),
            generation_id: generation_id.to_owned(),
            game_build: game_build.to_owned(),
            index,
            manifest_sha256,
        })
    }

    /// The export generation id.
    pub(crate) fn generation_id(&self) -> &str {
        &self.generation_id
    }

    /// The game build the generation was exported from.
    pub(crate) fn game_build(&self) -> &str {
        &self.game_build
    }

    /// The resource `guid`, its bytes verified against the manifest.
    pub(crate) fn resource(&self, guid: &str) -> Result<ExportedResource> {
        let Some(entry) = self.index.get(guid) else {
            bail!(
                "resource {guid} is not in the resource index of generation {}",
                self.generation_id
            );
        };
        let path = self.directory.join(&entry.resource_file);
        let (bytes, document) = read_json_file(&path)?;
        let sha256 = sha256_hex(&bytes);
        match self.manifest_sha256.get(&entry.resource_file) {
            Some(expected) if *expected == sha256 => {}
            Some(expected) => bail!(
                "{} hashes to {sha256} but the manifest records {expected}",
                path.display()
            ),
            None => bail!("{} is not listed in the manifest", entry.resource_file),
        }
        Ok(ExportedResource {
            guid: guid.to_owned(),
            resource_name: entry.resource_name.clone(),
            sha256,
            document,
        })
    }
}

/// The GUID inside a `{GUID}path` resource name.
pub(crate) fn guid_of_resource_name(resource_name: &str) -> Result<&str> {
    resource_name
        .strip_prefix('{')
        .and_then(|rest| rest.get(..16))
        .filter(|guid| guid.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .with_context(|| format!("{resource_name:?} does not start with a {{GUID}}"))
}

/// One verified resource document of the export.
pub(crate) struct ExportedResource {
    /// Sixteen uppercase hexadecimal digits.
    pub(crate) guid: String,
    /// `{GUID}path` as the engine names it.
    pub(crate) resource_name: String,
    /// SHA-256 of the exported file's bytes.
    pub(crate) sha256: String,
    document: Value,
}

impl ExportedResource {
    fn nodes(&self) -> impl Iterator<Item = &Value> {
        self.document["nodes"].as_array().into_iter().flatten()
    }

    /// Every node of class `class_name`, in export order.
    pub(crate) fn nodes_of_class(&self, class_name: &str) -> Vec<&Value> {
        self.nodes()
            .filter(|node| node["class_name"] == class_name)
            .collect()
    }

    /// The single node of class `class_name`.
    pub(crate) fn single_node(&self, class_name: &str) -> Result<&Value> {
        match self.nodes_of_class(class_name).as_slice() {
            [node] => Ok(node),
            nodes => bail!(
                "{} holds {} {class_name} nodes, expected one",
                self.resource_name,
                nodes.len()
            ),
        }
    }

    /// The value of property `name` on `node`.
    pub(crate) fn property<'a>(&self, node: &'a Value, name: &str) -> Result<&'a Value> {
        let property = &node["properties"][name];
        if property["status"] != "present" {
            bail!(
                "{} node {} has no present property {name}",
                self.resource_name,
                node["node_id"]
            );
        }
        Ok(&property["value"])
    }

    /// The numeric property `name` of the single `class_name` node.
    pub(crate) fn number(&self, class_name: &str, name: &str) -> Result<f64> {
        let node = self.single_node(class_name)?;
        self.property(node, name)?
            .as_f64()
            .with_context(|| format!("{} {class_name}.{name} is not a number", self.resource_name))
    }

    /// The nodes an array property of `node` refers to, in array order.
    pub(crate) fn referenced_nodes(&self, node: &Value, name: &str) -> Result<Vec<&Value>> {
        let by_id: BTreeMap<&str, &Value> = self
            .nodes()
            .filter_map(|node| Some((node["node_id"].as_str()?, node)))
            .collect();
        let references = self
            .property(node, name)?
            .as_array()
            .with_context(|| format!("{} property {name} is not a list", self.resource_name))?;
        references
            .iter()
            .map(|reference| {
                let id = reference["node_id"].as_str().unwrap_or_default();
                by_id
                    .get(id)
                    .copied()
                    .with_context(|| format!("{} refers to missing node {id}", self.resource_name))
            })
            .collect()
    }

    /// The first English display name the export resolved.
    pub(crate) fn display_name(&self) -> Result<String> {
        self.document["names"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|name| name["display_name_en"]["status"] == "present")
            .and_then(|name| name["display_name_en"]["value"].as_str())
            .map(str::to_owned)
            .with_context(|| format!("{} has no resolved English name", self.resource_name))
    }
}
