use chrono::Utc;
use cursive::reexports::log::error;
use home::home_dir;
use kube::api::GroupVersionKind;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

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
}

impl FsCache {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        let cache = match Self::load(name) {
            Ok(cache) => cache,
            Err(err) => {
                error!("Failed to load config: {}", err);
                Cache {
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    name: name.to_string(),
                    gvks: None
                }
            }
        };

        Ok(Self {
            name: name.to_string(),
            cache,
        })
    }

    fn get_home_dir() -> anyhow::Result<PathBuf> {
        let home_directory = home_dir().unwrap().join(".kgv").join("cache");
        fs::create_dir_all(home_directory.clone())?;
        Ok(home_directory)
    }

    fn get_current_config_file(name: &str) -> anyhow::Result<File> {
        let home = Self::get_home_dir()?;
        let cache_file = home.join(format!("{}.yaml", name));

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
        let file = Self::get_current_config_file(&self.name)?;
        serde_yaml::to_writer(file, &self.cache)?;
        Ok(())
    }

    pub fn load(name: &str) -> anyhow::Result<Cache> {
        let file = Self::get_current_config_file(name)?;
        let cache: Cache = serde_yaml::from_reader(file)?;
        Ok(cache)
    }

    fn update_cache_meta(&mut self) {
        self.cache.updated_at = Utc::now();
        self.cache.name = self.name.to_string();
    }
}

impl TryFrom<kube::Config> for FsCache {
    type Error = anyhow::Error;

    fn try_from(value: kube::Config) -> Result<Self, Self::Error> {
        let name = if let Some(q) = value.cluster_url.authority() {
            q.host().replace('.', "_")
        } else {
            Utc::now().to_string()
        };
        Self::new(&name)
    }
}
