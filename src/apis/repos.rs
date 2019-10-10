use crate::prelude::*;


#[derive(Debug, Deserialize, Serialize)]
pub struct ReposReq {
    repository_list : Vec<String>,
    modified_since : Option<String>,
    #[serde(flatten)]
    paging : Option<PagingInfo>
}