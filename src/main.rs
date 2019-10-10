#![allow(unused)]


use std::collections::{HashMap, HashSet};

use std::io;

use serde::{Deserialize, Serialize, Serializer};
use std::error::Error;

use env_logger::Logger;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, RwLock};


use regex::Match;

use std::iter::FromIterator;

pub mod prelude;
pub mod apis;
pub mod cache;

use crate::prelude::*;
use crate::cache::Cache;
use crate::apis::Api;
use crate::apis::updates::UpdatesApi;


fn main() -> std::io::Result<()> {
    env_logger::init();
    let cache = cache::load("data.dbm".to_string()).unwrap();

    let data = Data::new(RwLock::new(cache));
    println!("Serving");
    HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .register_data(data.clone())
            .data(web::JsonConfig::default().limit(1 * 1000 * 1000))
            .service(web::scope("/api/v1").configure(|c| {
                UpdatesApi::register(c);
            }))
    })

    .workers(1)
    //.backlog(1)
    .bind("127.0.0.1:8001")?
    .run()
}
