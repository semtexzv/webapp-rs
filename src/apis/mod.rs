use crate::prelude::*;
use crate::cache::Cache;

use std::sync::RwLock;

pub mod updates;
pub mod cve;
pub mod packages;
pub mod repos;

fn post_handler<A : Api>((req, body, cache) : (HttpRequest, Json<A::PostReqType>, Data<RwLock<Cache>>)) -> Json<A::RespType> {
    let cache = cache.get_ref().read().unwrap();
    let res = A::process_list(&cache,body.into_inner());
    res.map(Json).unwrap()
}

pub trait Api : Sized + 'static {
    type PostReqType : DeserializeOwned;
    type RespType : Serialize;
    const ENDPOINT_NAME : &'static str;

    fn process_list(cache : &Cache, req : Self::PostReqType) -> Result<Self::RespType>;


    fn register(app : &mut ServiceConfig) -> Result<()> {
        app.service(web::resource(Self::ENDPOINT_NAME).route(web::post().to(post_handler::<Self>)));
        Ok(())
    }
}