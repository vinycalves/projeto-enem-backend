INSERT INTO enem_topics (id, subject_code, parent_id, slug, name, description, sort_order) VALUES
(gen_random_uuid(), 'fisica', NULL, 'cinematica', 'Cinemática', 'Movimento, velocidade e aceleração.', 1),
(gen_random_uuid(), 'fisica', (SELECT id FROM enem_topics WHERE subject_code = 'fisica' AND slug = 'cinematica'), 'lancamento-obliquo', 'Lançamento oblíquo', 'Composição de movimentos horizontal e vertical.', 2),
(gen_random_uuid(), 'fisica', NULL, 'leis-newton', 'Leis de Newton', 'Força, massa, aceleração e interações.', 3),
(gen_random_uuid(), 'biologia', NULL, 'genetica', 'Genética', 'Hereditariedade, DNA e expressão gênica.', 1),
(gen_random_uuid(), 'biologia', (SELECT id FROM enem_topics WHERE subject_code = 'biologia' AND slug = 'genetica'), 'dna-replicacao', 'DNA e replicação', 'Estrutura da dupla hélice e pareamento de bases.', 2),
(gen_random_uuid(), 'biologia', NULL, 'citologia', 'Citologia', 'Células, organelas e metabolismo.', 3),
(gen_random_uuid(), 'quimica', NULL, 'ligacoes-quimicas', 'Ligações químicas', 'Interações entre átomos e moléculas.', 1),
(gen_random_uuid(), 'quimica', (SELECT id FROM enem_topics WHERE subject_code = 'quimica' AND slug = 'ligacoes-quimicas'), 'geometria-molecular', 'Geometria molecular', 'Forma espacial e polaridade das moléculas.', 2),
(gen_random_uuid(), 'geografia', NULL, 'cartografia', 'Cartografia', 'Escalas, projeções e representação do espaço.', 1),
(gen_random_uuid(), 'geografia', NULL, 'biomas', 'Biomas', 'Grandes conjuntos de ecossistemas e suas dinâmicas.', 2);

INSERT INTO visualizations (
    id, slug, subject_code, dimension, title, summary,
    learning_objectives, difficulty, params, common_mistakes, sources, component_key
) VALUES
(
    gen_random_uuid(),
    'lancamento-obliquo',
    'fisica',
    '2d',
    'Lançamento oblíquo',
    'Ajuste velocidade, ângulo e gravidade e veja a trajetória, os vetores e o alcance.',
    '["Relacionar ângulo de lançamento e alcance horizontal","Separar as componentes horizontal e vertical do movimento","Reconhecer que o alcance máximo, sem resistência do ar, ocorre em 45°"]'::jsonb,
    'intermediario',
    '[
      {"id":"velocidade","label":"Velocidade inicial","kind":"slider","min":10,"max":80,"step":1,"default":30,"unit":"m/s"},
      {"id":"angulo","label":"Ângulo de lançamento","kind":"slider","min":5,"max":85,"step":1,"default":45,"unit":"°"},
      {"id":"gravidade","label":"Gravidade","kind":"slider","min":1.6,"max":20,"step":0.1,"default":9.8,"unit":"m/s²"},
      {"id":"mostrarVetores","label":"Mostrar vetores de velocidade","kind":"toggle","default":true},
      {"id":"mostrarTrajetoria","label":"Mostrar trajetória","kind":"toggle","default":true}
    ]'::jsonb,
    '["Achar que o alcance máximo ocorre em 90°","Ignorar a componente horizontal da velocidade","Confundir a velocidade inicial com a velocidade no ponto mais alto"]'::jsonb,
    '["Modelo cinemático clássico sem resistência do ar; formulação padrão de lançamento oblíquo."]'::jsonb,
    'projectile-2d'
),
(
    gen_random_uuid(),
    'dna-3d',
    'biologia',
    '3d',
    'DNA em 3D',
    'Explore a dupla hélice, o pareamento de bases e a separação das fitas.',
    '["Reconhecer a estrutura de dupla hélice do DNA","Identificar o pareamento entre adenina-timina e citosina-guanina","Relacionar a estrutura do DNA à replicação"]'::jsonb,
    'intermediario',
    '[
      {"id":"rotacao","label":"Velocidade de rotação","kind":"slider","min":0,"max":2,"step":0.1,"default":0.6,"unit":"rad/s"},
      {"id":"mostrarPares","label":"Mostrar pares de bases","kind":"toggle","default":true},
      {"id":"destacarPar","label":"Destacar par de bases","kind":"select","default":"A-T","options":["A-T","C-G"]},
      {"id":"abrirHelice","label":"Abrir a hélice","kind":"slider","min":0,"max":1,"step":0.05,"default":0.1}
    ]'::jsonb,
    '["Achar que adenina pareia com guanina","Confundir DNA com RNA","Imaginar que as duas fitas são idênticas"]'::jsonb,
    '["Estrutura de dupla hélice descrita por Watson e Crick (1953); modelo didático simplificado."]'::jsonb,
    'dna-3d'
);

INSERT INTO visualization_topics (visualization_id, topic_id)
SELECT v.id, t.id
FROM visualizations v
JOIN enem_topics t ON t.subject_code = v.subject_code
WHERE v.slug = 'lancamento-obliquo' AND t.slug = 'lancamento-obliquo';

INSERT INTO visualization_topics (visualization_id, topic_id)
SELECT v.id, t.id
FROM visualizations v
JOIN enem_topics t ON t.subject_code = v.subject_code
WHERE v.slug = 'dna-3d' AND t.slug = 'dna-replicacao';

INSERT INTO molecules (id, slug, name, formula, atoms, bonds, metadata, sources) VALUES
(
    gen_random_uuid(),
    'agua',
    'Água',
    'H2O',
    '[
      {"element":"O","x":0.0,"y":0.0,"z":0.0},
      {"element":"H","x":0.757,"y":0.586,"z":0.0},
      {"element":"H","x":-0.757,"y":0.586,"z":0.0}
    ]'::jsonb,
    '[{"a":0,"b":1,"order":1},{"a":0,"b":2,"order":1}]'::jsonb,
    '{"geometry":"angular","bond_angle_deg":104.5,"note":"coordenadas didáticas aproximadas"}'::jsonb,
    '["PubChem CID 962"]'::jsonb
),
(
    gen_random_uuid(),
    'metano',
    'Metano',
    'CH4',
    '[
      {"element":"C","x":0.0,"y":0.0,"z":0.0},
      {"element":"H","x":0.629,"y":0.629,"z":0.629},
      {"element":"H","x":-0.629,"y":-0.629,"z":0.629},
      {"element":"H","x":-0.629,"y":0.629,"z":-0.629},
      {"element":"H","x":0.629,"y":-0.629,"z":-0.629}
    ]'::jsonb,
    '[{"a":0,"b":1,"order":1},{"a":0,"b":2,"order":1},{"a":0,"b":3,"order":1},{"a":0,"b":4,"order":1}]'::jsonb,
    '{"geometry":"tetraédrica","bond_angle_deg":109.5,"note":"coordenadas didáticas aproximadas"}'::jsonb,
    '["PubChem CID 297"]'::jsonb
),
(
    gen_random_uuid(),
    'co2',
    'Dióxido de carbono',
    'CO2',
    '[
      {"element":"C","x":0.0,"y":0.0,"z":0.0},
      {"element":"O","x":1.16,"y":0.0,"z":0.0},
      {"element":"O","x":-1.16,"y":0.0,"z":0.0}
    ]'::jsonb,
    '[{"a":0,"b":1,"order":2},{"a":0,"b":2,"order":2}]'::jsonb,
    '{"geometry":"linear","bond_angle_deg":180.0,"note":"coordenadas didáticas aproximadas"}'::jsonb,
    '["PubChem CID 280"]'::jsonb
);
