use crate::prelude::*;
use super::Api;
use crate::cache::Cache;
use std::collections::BTreeSet;

pub struct UpdatesApi;

#[derive(Debug, Deserialize, Clone)]
pub struct ModuleSpec {
    module_name: String,
    module_stream: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UpdatesReq {
    package_list: Vec<String>,

    repository_list: Option<Vec<String>>,
    modules_list: Option<Vec<ModuleSpec>>,
    releasever: Option<String>,
    basearch: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PkgUpdate {
    package: Nevra,
    erratum: String,

    repository : Option<String>,
    basearch : Option<String>,
    releasever : Option<String>

}

#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdatesPkgDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    summary : Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description : Option<String>,

    available_updates: Vec<PkgUpdate>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdatesData {
    update_list: Map<String, UpdatesPkgDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository_list: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    releasever: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    basearch: Option<String>,
}


impl UpdatesApi {
    fn related_products(cache: &Cache, original_repo_ids: &Set<u64>) -> Set<u64> {
        let mut product_ids = Set::default();
        for original_pkg_repo_id in original_repo_ids.iter() {
            if let Some(ref pid) = cache.repo_detail[original_pkg_repo_id].product_id {
                product_ids.insert(pid.clone());
            }
        }
        return product_ids;
    }

    fn valid_releasevers(cache: &Cache, original_repo_ids: &Set<u64>) -> Set<String> {
        let mut valid_releasevers = Set::default();
        for original_pkg_repo_id in original_repo_ids.iter() {
            if let Some(ref rv) = cache.repo_detail[original_pkg_repo_id].releasever {
                valid_releasevers.insert(rv.clone());
            }
        }
        return valid_releasevers;
    }

    fn build_nevra(cache: &Cache, update_pkg_id: u64) -> Nevra {
        let det = &cache.pkg_details[&update_pkg_id];
        let name = &cache.id_to_name[&det.name_id];
        let evr = &cache.id_to_evr[&det.evr_id];
        let arh = &cache.id_to_arch[&det.arch_id];
        return Nevra::from_name_evr_arch(name, evr.clone(), arh);
    }


    fn get_repositories(
        cache: &Cache,
        product_ids: &Set<u64>,
        update_pkg_id: u64,
        errata_ids: &[u64],
        available_repo_ids: &Set<u64>,
        valid_releasevers: &Set<String>,
    ) -> Set<u64> {
        let mut errata_repo_ids = Set::default();

        for errata_id in errata_ids {
            errata_repo_ids.extend(&cache.errataid_to_repoids[errata_id]);
        }

        let mut repo_ids = Set::from_iter(&cache.pkgid_to_repoids[&update_pkg_id])
                .intersection(&errata_repo_ids).map(|s| **s).collect::<Set<u64>>()
                .intersection(available_repo_ids).map(|s| *s).collect::<Set<u64>>();


        repo_ids.retain(|repo_id| {
            valid_releasevers.contains(
                cache.repo_detail[&repo_id]
                    .releasever
                    .as_ref()
                    .unwrap_or(&String::new())
                    .as_str(),
            ) && errata_repo_ids.contains(&repo_id)
        });

        return repo_ids;
    }

    fn process_updates(
        cache: &Cache,
        packages_to_process: &Map<&str, Nevra>,
        available_repo_ids: &Set<u64>,
        response: &mut UpdatesData,
    ) -> Result<(), Box<dyn Error>> {
        for (pkg, nevra) in packages_to_process.iter() {
            let nevra: &Nevra = nevra;
            let name_id = if let Some(x) = cache.name_to_id.get(&nevra.name) {
                x
            } else {
                continue;
            };
            let evr_id = cache.evr_to_id.get(&nevra.evr());
            let arch_id = cache.arch_to_id.get(&nevra.arch).ok_or("Arch id not found".to_string())?;

            // If nothing is found, use empty list
            let current_evr_idxs: &[_] = evr_id
                .and_then(|evr_id| cache.updates_index[&name_id].data.get(&evr_id))
                .map(|v| v.as_ref())
                .unwrap_or(&[][..]);

            if current_evr_idxs.is_empty() {
                //error!("package {:?} has no updates", name_id);
                continue ;
            }

            let mut current_nevra_pkg_id = None;

            for current_evr_idx in current_evr_idxs {
                //error!("current evr idx : => {:?}", current_evr_idx);

                let pkg_id = cache.updates[&name_id][*current_evr_idx as usize];
                let current_nevra_arch_id = &cache.pkg_details[&pkg_id].arch_id;

                //trace!("Package archs : {:?}, {:?}", current_nevra_arch_id, arch_id);
                if current_nevra_arch_id == arch_id {
                    current_nevra_pkg_id = Some(pkg_id);
                    break ;
                }
            }

            if current_nevra_pkg_id.is_none() {
                //error!("Package with NEVRA: {:?} not found", nevra);
                continue;
            } else {
                //error!("Package with NEVRA: {:?} Found", nevra);
            }

            let current_nevra_pkg_id = current_nevra_pkg_id.unwrap();

            let resp_pkg_detail = response.update_list.entry(pkg.to_string()).or_default();
            // TODO: for api version 1 only
            resp_pkg_detail.summary = cache.pkg_details[&current_nevra_pkg_id].summary.clone();
            resp_pkg_detail.description = cache.pkg_details[&current_nevra_pkg_id].desc.clone();

            let last_version_pkg_id = cache.updates[&name_id].last();
            if last_version_pkg_id == Some(&current_nevra_pkg_id) {
                //error!("Package is last, no updates");
                continue ;
            }

            let mut original_package_repo_ids = Set::default();

            if let Some(repoids) = cache.pkgid_to_repoids.get(&current_nevra_pkg_id) {
                original_package_repo_ids.extend(repoids.iter());
            }

            let product_ids = Self::related_products(cache, &original_package_repo_ids);
            let valid_releasevers = Self::valid_releasevers(cache, &original_package_repo_ids);
            //error!("Valid prods : {:#?}, valid vers : {:#?}", product_ids, valid_releasevers);
            let update_pkg_ids =
                &cache.updates[name_id][(*current_evr_idxs.last().unwrap() as usize) + 1..];

            for update_pkg_id in update_pkg_ids {
                //error!("Update pkg id : {:?}", update_pkg_id);
                let errata_ids = &cache.pkgid_to_errataids.get(update_pkg_id);
                if errata_ids.is_none() {
                    //error!("Filtering out :{:?}, no errata", update_pkg_id);
                    continue
                }
                let errata_ids = errata_ids.unwrap();

                let updated_nevra_arch_id = cache.pkg_details[update_pkg_id].arch_id;

                if updated_nevra_arch_id != *arch_id
                    && !cache.arch_compat[&arch_id].contains(&updated_nevra_arch_id)
                {
                    //error!("Filteroing out id : {:?}, wrong arch", update_pkg_id);
                    continue
                }

                let nevra = Self::build_nevra(cache, *update_pkg_id);
               //error!("update nvera: {:?}", nevra);
                for errata_id in errata_ids {
                    let mut repo_ids = Self::get_repositories(
                        cache,
                        &product_ids,
                        *update_pkg_id,
                        &[*errata_id],
                        &available_repo_ids,
                        &valid_releasevers,
                    );
                    //error!("Repoids  avail : {:#?} : {:#?}", available_repo_ids, repo_ids);

                    for repo_id in repo_ids {
                        let repo_det = &cache.repo_detail[&repo_id];
                        resp_pkg_detail.available_updates.push(PkgUpdate {
                            package: nevra.clone(),
                            erratum: cache.errataid_to_name[errata_id].clone(),
                            repository : Some(repo_det.label.clone()),
                            basearch : repo_det.basearch.clone(),
                            releasever : repo_det.releasever.clone()
                        })
                    }
                }
            }
        }
        Ok(())
    }

    fn process_repositories(
        cache: &Cache,
        data: &UpdatesReq,
        response: &mut UpdatesData,
    ) -> Set<u64> {
        let mut available_repo_ids = Vec::new();
        if let Some(ref repos) = data.repository_list {
            for repo in repos {
                if let Some(ids) = cache.repolabel_to_ids.get(repo) {
                    available_repo_ids.extend_from_slice(&ids)
                }
            }
            response.repository_list = Some(repos.clone());
        } else {
            available_repo_ids = cache.repo_detail.keys().map(|v| *v).collect::<Vec<_>>();
        }

        if let Some(ref releasever) = data.releasever {
            available_repo_ids.retain(|oid| {
                !(cache.repo_detail[oid].releasever.as_ref() == Some(&releasever)
                    || (cache.repo_detail[oid].releasever.is_none()
                    && cache.repo_detail[oid].url.contains(releasever)))
            });
            response.releasever = Some(releasever.clone())
        }

        if let Some(ref basearch) = data.basearch {
            available_repo_ids.retain(|oid| {
                !(cache.repo_detail[oid].basearch.as_ref() == Some(&basearch)
                    || (cache.repo_detail[oid].basearch.is_none()
                    && cache.repo_detail[oid].url.contains(basearch)))
            });
            response.basearch = Some(basearch.clone())
        }
        return Set::from_iter(available_repo_ids);
    }

    fn process_input_packages<'a>(
        cache: &'a Cache,
        data: &'a UpdatesReq,
        response: &mut UpdatesData,
    ) -> Map<&'a str, Nevra> {
        let mut filtered_pkgs_to_process = Map::default();

        for pkg in &data.package_list {
            let nevra = Nevra::from_str(&pkg).unwrap();
            if let Some(id) = cache.name_to_id.get(&nevra.name) {
                if let Some(up) = cache.updates_index.get(id) {
                    filtered_pkgs_to_process.insert(pkg.as_str(), nevra);
                }
            }
        }

        filtered_pkgs_to_process
    }
}

impl Api for UpdatesApi {
    type PostReqType = UpdatesReq;
    type RespType = UpdatesData;
    const ENDPOINT_NAME: &'static str = "/updates";


    fn process_list(cache: &Cache, data: Self::PostReqType) -> Result<Self::RespType, Box<dyn Error>> {
        let mut response = UpdatesData::default();
        let available_repo_ids = Self::process_repositories(cache, &data, &mut response);

        if let Some(ref modules_list) = data.modules_list {
            for m in modules_list {}
        }
        let mut packages_to_process = Self::process_input_packages(cache, &data, &mut response);
        Self::process_updates(
            cache,
            &packages_to_process,
            &available_repo_ids,
            &mut response,
        );
        Ok(response)
    }
}