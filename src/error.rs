use mobc_postgres::tokio_postgres;
use serde_derive::Serialize;
use std::convert::Infallible;
use thiserror::Error;
use warp::{http::StatusCode, Rejection, Reply};

#[derive(Error, Debug)]
pub enum Error {
    #[error("error getting connection from DB pool: {0}")]
    DBPoolError(mobc::Error<tokio_postgres::Error>),
    #[error("error executing DB query: {0}")]
    DBQueryError(#[from] tokio_postgres::Error),
    #[error("error creating table: {0}")]
    DBInitError(tokio_postgres::Error),
    #[error("error starting DB")]
    DBFatalError,
    #[error("error reading file: {0}")]
    ReadFileError(#[from] std::io::Error),
    #[error("Missing required fields in body")]
    MissingRequiredFields,
    #[error("User not found")]
    UserNotFound,
    #[error("Invalid search string")]
    InvalidSearch
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

impl warp::reject::Reject for Error {}

pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let code;
    let message;
    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not Found";
    } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
        code = StatusCode::BAD_REQUEST;
        message = "Invalid Body, but in a different way";
    } else if let Some(e) = err.find::<Error>() {
        match e {
            Error::DBQueryError(dberr) => {
                if dberr.code().unwrap() == &tokio_postgres::error::SqlState::UNIQUE_VIOLATION {
                    code = StatusCode::UNPROCESSABLE_ENTITY;
                    message = "Conflict";
                } else {
                    eprintln!("unhandled database error: {:?}", dberr);
                    code = StatusCode::INTERNAL_SERVER_ERROR;
                    message = "Internal Server Error";
                }
            }
            Error::InvalidSearch => {
                code = StatusCode::BAD_REQUEST;
                message = "Invalid Search";
            }
            Error::MissingRequiredFields => {
                code = StatusCode::UNPROCESSABLE_ENTITY;
                message = "Invalid Body";
            }
            _ => {
                eprintln!("unhandled application error: {:?}", err);
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "Internal Server Error";
            }
        }
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "Method Not Allowed";
    } else {
        eprintln!("unhandled error: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal Server Error";
    }

    let json = warp::reply::json(&ErrorResponse {
        message: message.into(),
    });

    Ok(warp::reply::with_status(json, code))
}
