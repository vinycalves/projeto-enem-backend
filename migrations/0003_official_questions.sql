ALTER TABLE questions ADD COLUMN external_id TEXT;
ALTER TABLE questions ADD COLUMN language TEXT;

CREATE UNIQUE INDEX questions_external_id_unique
    ON questions (external_id)
    WHERE external_id IS NOT NULL;

CREATE TABLE question_assets (
    id UUID PRIMARY KEY,
    question_id UUID NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('context', 'alternative')),
    option_label CHAR(1) CHECK (option_label IN ('A', 'B', 'C', 'D', 'E')),
    url TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX question_assets_question_idx ON question_assets (question_id, position);
