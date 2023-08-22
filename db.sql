CREATE TABLE IF NOT EXISTS Users (
    Id VARCHAR(255) PRIMARY KEY,
    Nome VARCHAR(255),
    Nascimento VARCHAR(255),
    Apelido VARCHAR(255) UNIQUE
);

CREATE TABLE IF NOT EXISTS Skills (
    Skill VARCHAR(255) PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS UserSkills (
    UserID VARCHAR(255),
    Skill VARCHAR(255),
    PRIMARY KEY (UserID, Skill),
    FOREIGN KEY (UserID) REFERENCES Users(ID) ON DELETE CASCADE,
    FOREIGN KEY (Skill) REFERENCES Skills(Skill) ON DELETE CASCADE
);
CREATE EXTENSION pg_trgm WITH SCHEMA pg_catalog;
CREATE INDEX skill_skill_gin_trgm_idx  ON Skills USING gin  (Skill gin_trgm_ops);
CREATE INDEX userskill_skill_gin_trgm_idx  ON UserSkills USING gin  (Skill gin_trgm_ops);
CREATE INDEX users_nome_gin_trgm_idx  ON Users USING gin  (Nome gin_trgm_ops);
CREATE INDEX users_apelido_gin_trgm_idx  ON Users USING gin  (Apelido gin_trgm_ops);