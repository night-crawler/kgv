use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use anyhow::Context;
use chrono::Utc;
use cursive::reexports::log::error;
use kube::api::GroupVersionKind;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Cache {
    gvks: Option<Vec<GroupVersionKind>>,
    name: String,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

pub struct FsCache {
    name: String,
    cache: Cache,
    cache_dir: Option<PathBuf>,
}

impl FsCache {
    pub fn new(cache_dir: Option<PathBuf>, name: &str) -> Self {
        let mut instance = Self {
            cache_dir,
            name: name.to_string(),
            cache: Cache {
                created_at: Utc::now(),
                updated_at: Utc::now(),
                name: name.to_string(),
                gvks: None,
            },
        };

        match instance.load() {
            Ok(cache) => instance.cache = cache,
            Err(err) => {
                error!("Failed to load cache: {}", err);
            }
        };

        instance
    }

    fn get_current_config_file(&self) -> anyhow::Result<File> {
        let dir = self.cache_dir.as_ref().context("Cache dir is not set")?;
        let cache_file = dir.join(format!("{}.yaml", self.name));

        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(cache_file)?;

        Ok(file)
    }

    pub fn set_gvks(&mut self, gvks: &[GroupVersionKind]) {
        self.cache.gvks = Some(gvks.to_vec());
        self.update_cache_meta();
    }

    pub fn get_gvks(&self) -> Option<Vec<GroupVersionKind>> {
        self.cache.gvks.clone()
    }

    pub fn dump(&self) -> anyhow::Result<()> {
        let file = self.get_current_config_file()?;
        serde_yaml::to_writer(file, &self.cache)?;
        Ok(())
    }

    pub fn load(&self) -> anyhow::Result<Cache> {
        let file = self.get_current_config_file()?;
        let cache: Cache = serde_yaml::from_reader(file)?;
        Ok(cache)
    }

    fn update_cache_meta(&mut self) {
        self.cache.updated_at = Utc::now();
        self.cache.name = self.name.to_string();
    }
}
