use crate::prelude::*;

use gnudbm::GdbmOpener;
use std::path::PathBuf;
use serde_aux::prelude::*;
use std::io;


#[derive(Debug, Deserialize, Clone, Hash, PartialOrd, PartialEq, Eq)]
pub struct Evr(
    #[serde(deserialize_with = "deserialize_number_from_string")] pub u64,
    pub String,
    pub String,
);

impl FromStr for Evr {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<_> = s.split(':').collect::<Vec<_>>();
        let release = parts.pop().unwrap();
        let version = parts.pop().unwrap();
        let epoch = parts.pop().unwrap();
        Ok(Evr(
            epoch.parse().unwrap(),
            version.to_string(),
            release.to_string(),
        ))
    }
}

#[derive(Debug, Deserialize, Clone, Hash, PartialOrd, PartialEq, Eq)]
pub struct NevraId(u64, u64, u64);

impl FromStr for NevraId {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<_> = s.split(':').collect::<Vec<_>>();
        let arch = parts.pop().unwrap();
        let evr = parts.pop().unwrap();
        let name = parts.pop().unwrap();
        Ok(NevraId(
            name.parse().unwrap(),
            evr.parse().unwrap(),
            arch.parse().unwrap(),
        ))
    }
}


#[derive(Debug, Deserialize, Clone, Hash)]
pub struct Package {
    pub name_id: u64,
    pub evr_id: u64,
    pub arch_id: u64,
    pub summary: Option<String>,
    pub desc: Option<String>,
    pub source_pkg_id: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, Hash)]
pub struct Cve {
    //id: String,
}

#[derive(Debug, Deserialize, Clone, Hash)]
pub struct Repo {
    pub label: String,
    pub name: String,
    pub url: String,
    pub basearch: Option<String>,
    pub releasever: Option<String>,
    pub product: Option<String>,
    pub product_id: Option<u64>,
    //revision : Option<u64>
}

#[derive(Debug, Deserialize, Clone, Hash)]
pub struct Errata {}

#[derive(Debug, Deserialize, Clone)]
pub struct UpdatesIndex {
    #[serde(flatten)]
    pub data: Map<u64, Vec<u64>>,
}


#[derive(Debug, Default)]
pub struct Cache {
    pub name_to_id: Map<String, u64>,
    pub id_to_name: Map<u64, String>,
    pub updates: Map<u64, Vec<u64>>,
    pub updates_index: Map<u64, UpdatesIndex>,
    pub evr_to_id: Map<Evr, u64>,
    pub id_to_evr: Map<u64, Evr>,
    pub arch_to_id: Map<String, u64>,
    pub id_to_arch: Map<u64, String>,
    pub arch_compat: Map<u64, Vec<u64>>,

    pub pkg_details: Map<u64, Package>,
    pub nevra_to_pkgid: Map<NevraId, u64>,
    pub repo_detail: Map<u64, Repo>,
    pub repolabel_to_ids: Map<String, Vec<u64>>,
    pub productid_to_repoids: Map<u64, Vec<u64>>,
    pub pkgid_to_repoids: Map<u64, Vec<u64>>,
    pub errataid_to_name: Map<u64, String>,
    pub pkgid_to_errataids: Map<u64, Vec<u64>>,
    pub errataid_to_repoids: Map<u64, Vec<u64>>,
    pub cve_detail: Map<String, Cve>,
    pub dbchange: Map<String, String>,
    pub errata_detail: Map<String, Errata>,
    pub pkgerrata_to_module: Map<String, String>,
    pub modulename_to_id: Map<String, String>,
    pub src_pkg_id_to_pkg_ids: Map<String, Vec<u64>>,
}

pub fn load(name: String) -> Result<Cache, Box<dyn Error>> {
    let file = PathBuf::from(name);
    let db = GdbmOpener::new().readonly(&file).expect("Opening failed");

    let mut cache = Cache::default();
    for (key, data) in db.iter() {
        let kstr = std::str::from_utf8(key.as_bytes()).unwrap();
        let (key, id) = kstr.split_at(kstr.find(':').unwrap());
        let (_, id) = id.split_at(1);
        let data = data.as_bytes();

        match key {
            "packagename2id" => {
                cache
                    .name_to_id
                    .insert(id.to_owned(), pickle::from_slice::<u64>(data).unwrap());
                //println!("{:?} = {:?}", id, cache.name_to_id[id])
            }
            "id2packagename" => {
                cache
                    .id_to_name
                    .insert(id.parse()?, pickle::from_slice::<String>(data)?);
            }
            "updates" => {
                cache.updates.insert(id.parse()?, pickle::from_slice(data)?);
            }
            "updates_index" => {
                cache
                    .updates_index
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "evr2id" => {
                cache
                    .evr_to_id
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "id2evr" => {
                cache
                    .id_to_evr
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "arch2id" => {
                cache
                    .arch_to_id
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "id2arch" => {
                cache
                    .id_to_arch
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "arch_compat" => {
                cache
                    .arch_compat
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "package_details" => {
                cache
                    .pkg_details
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "nevra2pkgid" => {
                cache
                    .nevra_to_pkgid
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "repo_detail" => {
                let id = id.parse()?;
                cache.repo_detail.insert(id, pickle::from_slice(data)?);
            }
            "repolabel2ids" => {
                cache
                    .repolabel_to_ids
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "productid2repoids" => {
                cache
                    .productid_to_repoids
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "pkgid2repoids" => {
                cache
                    .pkgid_to_repoids
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "errataid2name" => {
                cache
                    .errataid_to_name
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "pkgid2errataids" => {
                cache
                    .pkgid_to_errataids
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "errataid2repoids" => {
                cache
                    .errataid_to_repoids
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "cve_detail" => {
                cache
                    .cve_detail
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "dbchange" => {
                //cache.dbchange.insert(id.parse()?, pickle::from_slice(data)?);
            }
            "errata_detail" => {
                cache
                    .errata_detail
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "pkgerrata2module" => {
                // TODO:
            }
            "modulename2id" => {
                //println!("Id: {:?}", id);
                cache
                    .modulename_to_id
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            "src_pkg_id2pkg_ids" => {
                cache
                    .src_pkg_id_to_pkg_ids
                    .insert(id.parse()?, pickle::from_slice(data)?);
            }
            other => panic!("Table {:?} not implemented", other),
        }
    }

    Ok(cache)
}
