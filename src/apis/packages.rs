use crate::prelude::*;

pub struct CveApi;


#[derive(Debug, Deserialize, Serialize)]
pub struct PackagesReq {
    package_list : Vec<String>,
}