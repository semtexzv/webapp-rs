use crate::prelude::*;

pub struct CveApi;

#[derive(Debug, Deserialize, Serialize)]
pub struct CveReq {
    cve_list : Vec<String>,
    modified_since : Option<String>,
    published_since : Option<String>,
    #[serde(flatten)]
    paging : Option<PagingInfo>,
    rh_only : Option<bool>,
}