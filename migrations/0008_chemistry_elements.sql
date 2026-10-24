CREATE TABLE chemistry_elements (
    atomic_number INTEGER PRIMARY KEY,
    symbol TEXT NOT NULL UNIQUE,
    name_pt TEXT NOT NULL,
    atomic_mass DOUBLE PRECISION NOT NULL CHECK (atomic_mass > 0),
    group_number INTEGER CHECK (group_number BETWEEN 1 AND 18),
    period INTEGER NOT NULL CHECK (period BETWEEN 1 AND 7),
    electronegativity DOUBLE PRECISION,
    atomic_radius_pm DOUBLE PRECISION,
    electron_configuration TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL,
    standard_state TEXT,
    cpk_hex TEXT,
    sources JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX chemistry_elements_symbol_idx ON chemistry_elements (symbol);
