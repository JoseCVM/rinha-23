use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct User {
    pub id: String,
    pub apelido: String,
    pub nome: String,
    pub nascimento: String,
    pub stack: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct PossibleCreateUserRequest {
    pub apelido: Option<String>,
    pub nome: Option<String>,
    pub nascimento: Option<String>,
    pub stack: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub apelido: String,
    pub nome: String,
    pub nascimento: String,
    pub stack: Option<Vec<String>>,
}

impl From<PossibleCreateUserRequest> for CreateUserRequest {
    fn from(body: PossibleCreateUserRequest) -> Self {
        CreateUserRequest {
            apelido: body.apelido.unwrap(),
            nome: body.nome.unwrap(),
            nascimento: body.nascimento.unwrap(),
            stack: body.stack,
        }
    }
}