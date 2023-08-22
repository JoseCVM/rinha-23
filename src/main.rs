use std::time::Duration;
use data::User;
use mobc::{Connection, Pool};
use mobc_postgres::{tokio_postgres, PgConnectionManager};
use std::convert::Infallible;
use tokio_postgres::NoTls;
use warp::{Filter, Rejection};

mod data;
mod db;
mod error;
mod handler;

type Result<T> = std::result::Result<T, Rejection>;
type DBCon = Connection<PgConnectionManager<NoTls>>;
type DBPool = Pool<PgConnectionManager<NoTls>>;
use moka::future::Cache;

#[tokio::main]
async fn main() {
    let db_pool = db::create_pool().expect("database pool can be created");
    let cache = Cache::builder().max_capacity(150000).time_to_idle(Duration::new(1200, 0)).build();
    db::init_db(&db_pool)
        .await
        .expect("database can be initialized");

    let health_route = warp::path!("health")
        .and(with_db(db_pool.clone()))
        .and_then(handler::health_handler);
    let count_route = warp::path!("contagem-pessoas")
        .and(with_db(db_pool.clone()))
        .and_then(handler::count_users);
    let users = warp::path("pessoas");
    let users_routes = users
        .and(warp::get())
        .and(warp::path::param())
        .and(with_db(db_pool.clone()))
        .and(with_cache(cache.clone()))
        .and_then(handler::fetch_user_by_id_handler)
        .or(users
            .and(warp::post())
            .and(warp::body::json())
            .and(with_db(db_pool.clone()))
            .and(with_cache(cache.clone()))
            .and_then(handler::create_user_handler))
        .or(users
            .and(warp::get())
            .and(warp::query())
            .and(with_db(db_pool.clone()))
            .and_then(handler::search_users_handler));

    let routes = health_route
        .or(count_route)
        .or(users_routes)
        .with(warp::cors().allow_any_origin())
        .recover(error::handle_rejection);

    warp::serve(routes).run(([0, 0, 0, 0], 8000)).await;
}

fn with_db(db_pool: DBPool) -> impl Filter<Extract = (DBPool,), Error = Infallible> + Clone {
    warp::any().map(move || db_pool.clone())
}

fn with_cache(cache: Cache<String, User>) -> impl Filter<Extract = (Cache<String, User>,), Error = Infallible> + Clone {
    warp::any().map(move || cache.clone())
}
