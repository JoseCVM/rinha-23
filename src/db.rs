use crate::{data::*, error, error::Error::*, DBCon, DBPool};
use mobc::Pool;
use mobc_postgres::{
    tokio_postgres::{self, types::ToSql},
    PgConnectionManager,
};
use std::fs;
use std::str::FromStr;
use std::time::Duration;
use tokio_postgres::{Config, Error, NoTls, Row};
use uuid::Uuid;

type Result<T> = std::result::Result<T, error::Error>;

const DB_POOL_MAX_OPEN: u64 = 32;
const DB_POOL_MAX_IDLE: u64 = 8;
const DB_POOL_TIMEOUT_SECONDS: u64 = 15;
const INIT_SQL: &str = "./db.sql";

pub async fn init_db(db_pool: &DBPool) -> Result<()> {
    let init_file = fs::read_to_string(INIT_SQL)?;
    let con = get_db_con(db_pool).await?;
    con.batch_execute(init_file.as_str())
        .await
        .map_err(DBInitError)?;
    Ok(())
}

pub async fn get_db_con(db_pool: &DBPool) -> Result<DBCon> {
    db_pool.get().await.map_err(DBPoolError)
}

pub fn create_pool() -> std::result::Result<DBPool, mobc::Error<Error>> {
    let config = Config::from_str("postgres://postgres@localhost:7878/postgres")?;

    let manager = PgConnectionManager::new(config, NoTls);
    Ok(Pool::builder()
        .max_open(DB_POOL_MAX_OPEN)
        .max_idle(DB_POOL_MAX_IDLE)
        .get_timeout(Some(Duration::from_secs(DB_POOL_TIMEOUT_SECONDS)))
        .build(manager))
}

pub async fn search_users(db_pool: &DBPool, search: String) -> Result<Vec<User>> {
    let con = get_db_con(db_pool).await?;

    let search_pattern = format!("%{}%", search);

    let query = r#"
    SELECT users.id, users.apelido, users.nome, users.nascimento, ARRAY_AGG(Skills.Skill) as skills
    FROM users
    LEFT JOIN UserSkills ON users.id = UserSkills.UserID
    LEFT JOIN Skills ON UserSkills.Skill = Skills.Skill
    WHERE users.nome LIKE $1
        OR users.apelido LIKE $1
        OR Skills.Skill LIKE $1
    GROUP BY users.id
    LIMIT 50
"#;

    let rows = con
        .query(query, &[&search_pattern])
        .await
        .map_err(DBQueryError)?;

    let users : Vec<User> = rows.iter().map(|row| row_to_user(&row)).collect();
    Ok(users)
}

pub async fn fetch_user_by_id(db_pool: &DBPool, user_id: &String) -> Result<Option<User>> {
    println!("fetch_user_by_id: {}", user_id);
    let con = get_db_con(db_pool).await?;

    let query = r#"
        SELECT users.*, array_agg(Skills.Skill) AS skills
        FROM users
        LEFT JOIN UserSkills ON users.id = UserSkills.UserID
        LEFT JOIN Skills ON UserSkills.Skill = Skills.Skill
        WHERE users.id = $1
        GROUP BY users.id
    "#;

    let row = con
        .query_opt(query, &[&user_id])
        .await
        .map_err(DBQueryError)?;

    if let Some(row) = row {
        let user = row_to_user(&row); // Assuming a custom function to map row to User with skills
        Ok(Some(user))
    } else {
        Ok(None)
    }
}

pub async fn create_user(db_pool: &DBPool, body: CreateUserRequest) -> Result<User> {
    let mut con = get_db_con(db_pool).await?;
    let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, &body.apelido.as_bytes()).to_string();

    let transaction = con.transaction().await?;
    // Insert user
    let insert_user_query = format!(
        "INSERT INTO Users (id, apelido, nome, nascimento) VALUES ($1, $2, $3, $4) RETURNING *"
    );
    transaction.query_one(
        insert_user_query.as_str(),
        &[&id, &body.apelido, &body.nome, &body.nascimento],
    )
    .await
    .map_err(DBQueryError)?;

    // Insert skills
    match &body.stack {
        Some(skills) => {
            let query = "INSERT INTO UserSkills (UserID, Skill) VALUES ($1, $2)";

            for skill in skills {
                transaction
                    .execute(query, &[&id, &skill])
                    .await
                    .map_err(DBQueryError)?;
            }

            let params = (1..=skills.len())
                .map(|i| format!("${}", i + 1))
                .collect::<Vec<String>>()
                .join(", ");
            let associate_skills_query = format!(
                "INSERT INTO UserSkills (UserID, Skill) SELECT $1, Skill FROM Skills WHERE Skill IN ({})",
                params
            );
            let mut param_values: Vec<&(dyn ToSql + Sync)> = Vec::new();
            param_values.push(&id);
            param_values.extend(skills.iter().map(|s| s as &(dyn ToSql + Sync)));
            transaction.execute(associate_skills_query.as_str(), &param_values)
                .await
                .map_err(DBQueryError)?;
        }
        None => {}
    }
    transaction.commit().await?;

    Ok(User {
        id,
        apelido: body.apelido,
        nome: body.nome,
        nascimento: body.nascimento,
        stack: body.stack,
    })
}

fn row_to_user(row: &Row) -> User {
    let skill: Vec<String> = row.try_get("skills").unwrap_or(Vec::new());
    let id: String = row.get("id");
    let apelido: String = row.get("apelido");
    let nome: String = row.get("nome");
    let nascimento: String = row.get("nascimento");
    User {
        id,
        apelido,
        nome,
        nascimento,
        stack: Some(skill),
    }
}
