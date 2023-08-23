use crate::{data::*, db, error::Error::*, DBPool, Result};
use moka::future::Cache;
use serde_derive::Deserialize;
use uuid::Uuid;
use warp::{ 
    http::{StatusCode, Response},
    reject,
    reply::{json, WithStatus},
    Filter, Reply,
};

#[derive(Deserialize)]
pub struct SearchQuery {
    t: Option<String>,
}

pub async fn health_handler(db_pool: DBPool) -> Result<impl Reply> {
    let db = db_pool.get().await.unwrap();
    db.execute("SELECT 1", &[])
        .await
        .map_err(|e| reject::custom(DBQueryError(e)))?;
    Ok(StatusCode::OK)
}

pub async fn create_user_handler(
    body: PossibleCreateUserRequest,
    db_pool: DBPool,
    cache: Cache<String, User>,
) -> Result<impl Reply> {
    if body.apelido.is_none() || body.nome.is_none() || body.nascimento.is_none() {
        return Err(reject::custom(MissingRequiredFields));
    }
    let apelido = body.apelido.clone().unwrap();
    let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, apelido.as_bytes()).to_string();
    match cache.get(&id) {
        Some(_) => {
            //println!("User already exists in cache with id {}", apelido);
            return Err(reject::custom(UserAlreadyExists));
        }
        None => {
            // match check_remote_cache(&id).await {
            //     true => {
            //         //println!("User already exists in remote cache with id {}", user_id);
            //         return Err(reject::custom(UserAlreadyExists))
            //     }
            //     false => {
            //         //println!("User does not exist in remote cache with id {}", user_id);
            //     }
            // }
        }
    };
    let create_user_request: CreateUserRequest = {
        CreateUserRequest {
            apelido: apelido,
            nome: body.nome.unwrap(),
            nascimento: body.nascimento.unwrap(),
            stack: body.stack,
        }
    };
    match db::create_user(&db_pool, create_user_request).await {
        Ok(user) => {
            cache.insert(user.id.clone(), user.clone()).await;
            let reply = warp::reply::json(&user);
            let reply = warp::reply::with_status(reply, StatusCode::CREATED);
            let reply = warp::reply::with_header(reply, "Location", format!("/pessoas/{}", user.id));
            Ok(reply.into_response())
        }
        Err(e) => Err(reject::custom(e)),
    }
}

pub async fn fetch_user_by_id_handler(
    id: String,
    db_pool: DBPool,
    cache: Cache<String, User>,
) -> Result<impl Reply> {
    match cache.get(&id) {
        Some(user) => return Ok(json(&user)),
        None => match db::fetch_user_by_id(&db_pool, &id).await {
            Ok(Some(user)) => {
                cache.insert(user.id.clone(), user.clone()).await;
                return Ok(json(&user));
            }
            Ok(None) => return Err(reject::custom(UserNotFound)),
            Err(e) => return Err(reject::custom(e)),
        },
    }
}

pub async fn count_users(db_pool: DBPool) -> Result<impl Reply> {
    Ok(json(
        &db::count_users(&db_pool)
            .await
            .map_err(|e| reject::custom(e))?,
    ))
}

pub async fn search_users_handler(query: SearchQuery, db_pool: DBPool) -> Result<impl Reply> {
    match query.t {
        None => Err(reject::custom(InvalidSearch)),
        Some(query) => Ok(json(
            &db::search_users(&db_pool, query)
                .await
                .map_err(|e| reject::custom(e))?,
        )),
    }
}
