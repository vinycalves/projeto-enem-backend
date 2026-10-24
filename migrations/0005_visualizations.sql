CREATE TABLE enem_topics (
    id UUID PRIMARY KEY,
    subject_code TEXT NOT NULL REFERENCES subjects(code) ON DELETE CASCADE,
    parent_id UUID REFERENCES enem_topics(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    UNIQUE (subject_code, slug)
);

CREATE INDEX enem_topics_subject_idx ON enem_topics (subject_code, sort_order);
CREATE INDEX enem_topics_parent_idx ON enem_topics (parent_id);

CREATE TABLE visualizations (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    subject_code TEXT NOT NULL REFERENCES subjects(code) ON DELETE RESTRICT,
    dimension TEXT NOT NULL CHECK (dimension IN ('2d', '3d')),
    title TEXT NOT NULL,
    summary TEXT NOT NULL CHECK (char_length(summary) <= 200),
    learning_objectives JSONB NOT NULL DEFAULT '[]'::jsonb,
    difficulty TEXT NOT NULL CHECK (difficulty IN ('basico', 'intermediario', 'avancado')),
    params JSONB NOT NULL DEFAULT '[]'::jsonb,
    common_mistakes JSONB NOT NULL DEFAULT '[]'::jsonb,
    sources JSONB NOT NULL DEFAULT '[]'::jsonb,
    component_key TEXT NOT NULL,
    published BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX visualizations_filters_idx
    ON visualizations (subject_code, dimension, difficulty, published);

CREATE TABLE visualization_topics (
    visualization_id UUID NOT NULL REFERENCES visualizations(id) ON DELETE CASCADE,
    topic_id UUID NOT NULL REFERENCES enem_topics(id) ON DELETE CASCADE,
    PRIMARY KEY (visualization_id, topic_id)
);

CREATE TABLE molecules (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    formula TEXT NOT NULL,
    atoms JSONB NOT NULL,
    bonds JSONB NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    sources JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE geo_layers (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    geojson JSONB NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    sources JSONB NOT NULL DEFAULT '[]'::jsonb,
    level TEXT NOT NULL DEFAULT 'simplified',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
