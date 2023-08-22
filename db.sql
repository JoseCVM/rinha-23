CREATE TABLE IF NOT EXISTS Users (
    Id UUID PRIMARY KEY,
    Nome VARCHAR(255),
    Nascimento VARCHAR(255),
    Apelido VARCHAR(255) UNIQUE
);

CREATE TABLE IF NOT EXISTS Skills (
    SkillId SERIAL PRIMARY KEY,
    Skill VARCHAR(255) UNIQUE
);

CREATE TABLE IF NOT EXISTS UserSkills (
    UserID UUID,
    SkillId INTEGER,
    PRIMARY KEY (UserID, SkillId),
    FOREIGN KEY (UserID) REFERENCES Users(Id) ON DELETE CASCADE,
    FOREIGN KEY (SkillId) REFERENCES Skills(SkillId) ON DELETE CASCADE
);

CREATE EXTENSION pg_trgm;
CREATE INDEX skill_skill_gin_trgm_idx    ON Skills USING gin  (Skill gin_trgm_ops);
CREATE INDEX users_nome_gin_trgm_idx  ON Users USING gin  (Nome gin_trgm_ops);
CREATE INDEX users_apelido_gin_trgm_idx  ON Users USING gin  (Apelido gin_trgm_ops);