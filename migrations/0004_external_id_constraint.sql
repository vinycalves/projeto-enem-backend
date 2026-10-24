DROP INDEX IF EXISTS questions_external_id_unique;

CREATE UNIQUE INDEX questions_external_id_unique ON questions (external_id);
