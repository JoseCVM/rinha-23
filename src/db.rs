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

const DB_POOL_MAX_OPEN: u64 = 40;
const DB_POOL_TIMEOUT_SECONDS: u64 = 15;
const DB_POOL_MAX_IDLE: u64 = 20;
const DB_POOL_EXPIRE_SECONDS: u64 = 30;
const INIT_SQL: &str = "./db.sql";

pub async fn init_db(db_pool: &DBPool) -> Result<()> {
    let init_file = fs::read_to_string(INIT_SQL)?;

    let con = db_pool.get().await.unwrap();

    match con.batch_execute(init_file.as_str()).await {
        Ok(_) => return Ok(()),
        Err(e) => {
            eprintln!("Failed to initialize database: {}", e);
            return Ok(());
        }
    }
}


pub fn create_pool() -> std::result::Result<DBPool, mobc::Error<Error>> {
    let config = Config::from_str("postgres://postgres@db:5432/postgres")?;

    let manager = PgConnectionManager::new(config, NoTls);
    Ok(Pool::builder()
        .max_open(DB_POOL_MAX_OPEN)
        .max_idle(DB_POOL_MAX_IDLE)
        .max_lifetime(Some(Duration::from_secs(DB_POOL_EXPIRE_SECONDS)))
        .get_timeout(None)
        .build(manager))
}

pub async fn count_users(db_pool: &DBPool) -> Result<i64> {
    let con = db_pool.get().await.unwrap();
    let rows = con
        .query("SELECT COUNT(*) FROM users", &[])
        .await
        .map_err(DBQueryError)?;
    let count: i64 = rows[0].get(0);

    Ok(count)
}

pub async fn search_users(db_pool: &DBPool, search: String) -> Result<Vec<User>> {
    let con = db_pool.get().await.unwrap();

    //let search_pattern = format!("%{}%", search);

    let query = r#"
    SELECT users.id, users.apelido, users.nome, users.nascimento, ARRAY_AGG(Skills.Skill) as skills
    FROM users
    LEFT JOIN UserSkills ON users.id = UserSkills.UserID
    LEFT JOIN Skills ON UserSkills.SkillId = Skills.SkillId
    WHERE users.nome LIKE $1
        OR users.apelido LIKE $1
        OR Skills.Skill LIKE $1
    GROUP BY users.id
    LIMIT 50
"#;
    let rows = con.query(query, &[&search]).await.map_err(DBQueryError)?;

    let users: Vec<User> = rows.iter().map(|row| row_to_user(&row)).collect();
    Ok(users)
}

pub async fn fetch_user_by_id(db_pool: &DBPool, user_id: &String) -> Result<Option<User>> {
    let user_id = Uuid::parse_str(user_id).map_err(|_| InvalidSearch)?;
    let con = db_pool.get().await.unwrap();

    let query = r#"
        SELECT users.*, array_agg(Skills.Skill) AS skills
        FROM users
        LEFT JOIN UserSkills ON users.id = UserSkills.UserID
        LEFT JOIN Skills ON UserSkills.SkillId = Skills.SkillId
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
    let con = db_pool.get().await.unwrap();

    let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, &body.apelido.as_bytes());

    // Insert user
    let insert_user_query =
        format!("INSERT INTO Users (id, apelido, nome, nascimento) VALUES ($1, $2, $3, $4)");
    match con
        .execute(
            insert_user_query.as_str(),
            &[&id, &body.apelido, &body.nome, &body.nascimento],
        )
        .await
    {
        Ok(_) => {}
        Err(e) => {
            if e.code() == Some(&tokio_postgres::error::SqlState::UNIQUE_VIOLATION) {
                return Err(UserAlreadyExists);
            } else {
                return Err(DBQueryError(e));
            }
        }
    }
    // Insert skills
    match &body.stack {
        Some(skills) => {
            let mut sql_batch = "DO $$ BEGIN\n".to_owned();
            for skill in skills {
                sql_batch.push_str(&format!(
                    "INSERT INTO Skills (Skill) VALUES ('{}') ON CONFLICT DO NOTHING;\n",
                    skill
                ));
            }

            let skill_values = skills
                .iter()
                .map(|s| format!("'{}'", s))
                .collect::<Vec<String>>()
                .join(", ");
            sql_batch.push_str(&format!(
                "INSERT INTO UserSkills (UserID, SkillId) SELECT '{}', SkillId FROM Skills WHERE Skill IN ({}) ON CONFLICT DO NOTHING;\n",
                id,
                skill_values
            ));

            sql_batch.push_str("END $$;\n");
            con.batch_execute(sql_batch.as_str()).await.unwrap();
        }
        None => {}
    }

    Ok(User {
        id: id.to_string(),
        apelido: body.apelido,
        nome: body.nome,
        nascimento: body.nascimento,
        stack: body.stack,
    })
}

fn row_to_user(row: &Row) -> User {
    let skill: Vec<String> = row.try_get("skills").unwrap_or(Vec::new());
    let id: Uuid = row.get("id");
    let id = id.to_string();
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
