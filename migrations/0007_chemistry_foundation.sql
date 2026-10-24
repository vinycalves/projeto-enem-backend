ALTER TABLE visualizations
    ADD COLUMN model_kind TEXT NOT NULL DEFAULT 'diagrama'
        CHECK (model_kind IN ('simulacao', 'animacao_didatica', 'diagrama')),
    ADD COLUMN dimension_rationale TEXT NOT NULL DEFAULT 'Representação escolhida pela clareza didática.',
    ADD COLUMN tier TEXT NOT NULL DEFAULT 'T1'
        CHECK (tier IN ('T1', 'T2', 'T3')),
    ADD COLUMN simplifications JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN enem_notes JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN quiz JSONB NOT NULL DEFAULT '[]'::jsonb;

UPDATE visualizations
SET model_kind = 'simulacao',
    dimension_rationale = 'Trajetória, vetores e alcance são relações quantitativas; o plano 2D favorece a leitura dos eixos e do movimento.',
    tier = 'T1',
    simplifications = '["Sem resistência do ar; gravidade uniforme; partícula puntiforme.","Escalas de tempo e espaço são didáticas."]'::jsonb,
    enem_notes = '["Separar movimento horizontal uniforme e vertical uniformemente variado.","Relacionar alcance, ângulo e velocidade inicial.","Reconhecer o alcance máximo em 45° quando não há resistência do ar."]'::jsonb
WHERE slug = 'lancamento-obliquo';

UPDATE visualizations
SET model_kind = 'animacao_didatica',
    dimension_rationale = 'A geometria espacial da dupla hélice é o conceito central; a cena 3D permite observar o enrolamento e o pareamento.',
    tier = 'T1',
    simplifications = '["Estrutura esquemática, sem escalas atômicas reais.","O giro e a abertura da hélice são recursos didáticos, não o mecanismo de replicação."]'::jsonb,
    enem_notes = '["Reconhecer a dupla hélice e o pareamento A-T/C-G.","Relacionar a estrutura do DNA à replicação semiconservativa."]'::jsonb
WHERE slug = 'dna-3d';

CREATE TABLE chemistry_reactions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    classification TEXT NOT NULL,
    reaction JSONB NOT NULL,
    sources JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE chemistry_datasets (
    name TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    conditions TEXT NOT NULL,
    data JSONB NOT NULL,
    sources JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO enem_topics (id, subject_code, parent_id, slug, name, description, sort_order) VALUES
(gen_random_uuid(), 'quimica', NULL, 'termoquimica', 'Termoquímica', 'Energia, entalpia e variações térmicas das reações.', 10),
(gen_random_uuid(), 'quimica', NULL, 'equilibrio-quimico', 'Equilíbrio químico', 'Reversibilidade, constante de equilíbrio e perturbações.', 11),
(gen_random_uuid(), 'quimica', NULL, 'equilibrio-ionico', 'Equilíbrio iônico e pH', 'Ácidos, bases, pH, titulação e soluções tampão.', 12)
ON CONFLICT (subject_code, slug) DO NOTHING;

INSERT INTO enem_topics (id, subject_code, parent_id, slug, name, description, sort_order)
SELECT gen_random_uuid(), 'quimica', id, 'combustao', 'Combustão e entalpia', 'Combustão completa, balanceamento e energia liberada.', 1
FROM enem_topics
WHERE subject_code = 'quimica' AND slug = 'termoquimica'
ON CONFLICT (subject_code, slug) DO NOTHING;

INSERT INTO enem_topics (id, subject_code, parent_id, slug, name, description, sort_order)
SELECT gen_random_uuid(), 'quimica', id, 'n2o4-no2', 'Equilíbrio N₂O₄/NO₂', 'Dissociação do tetraóxido de dinitrogênio e deslocamento do equilíbrio.', 1
FROM enem_topics
WHERE subject_code = 'quimica' AND slug = 'equilibrio-quimico'
ON CONFLICT (subject_code, slug) DO NOTHING;

INSERT INTO enem_topics (id, subject_code, parent_id, slug, name, description, sort_order)
SELECT gen_random_uuid(), 'quimica', id, 'titulacao', 'Titulação ácido-base', 'Curvas de neutralização, ponto de equivalência e indicadores.', 1
FROM enem_topics
WHERE subject_code = 'quimica' AND slug = 'equilibrio-ionico'
ON CONFLICT (subject_code, slug) DO NOTHING;

INSERT INTO visualizations (
    id, slug, subject_code, dimension, title, summary,
    learning_objectives, difficulty, params, common_mistakes, sources, component_key,
    model_kind, dimension_rationale, tier, simplifications, enem_notes, quiz
) VALUES
(
    gen_random_uuid(),
    'combustao-metano',
    'quimica',
    '2d',
    'Combustão do metano',
    'Acompanhe a combustão completa do metano, confira o balanceamento e interprete a energia liberada.',
    '["Balancear a combustão completa do metano","Interpretar um diagrama de entalpia e o sinal de ΔH","Relacionar a estequiometria da reação à energia liberada"]'::jsonb,
    'intermediario',
    '[
      {"id":"etapa","label":"Etapa da animação","kind":"slider","min":0,"max":1,"step":0.01,"default":0,"unit":""},
      {"id":"mostrarEnergia","label":"Mostrar diagrama de entalpia","kind":"toggle","default":true}
    ]'::jsonb,
    '["Esquecer o coeficiente 2 do O₂","Somar entalpias sem multiplicar pelos coeficientes","Usar água gasosa e água líquida como se tivessem a mesma entalpia"]'::jsonb,
    '["NIST Chemistry WebBook, Methane (CID 74828), termoquímica a 298,15 K.","NIST-JANAF Thermochemical Tables, 4th ed. (1998).","IUPAC. Quantities, Units and Symbols in Physical Chemistry, 3rd ed."]'::jsonb,
    'combustion-2d',
    'animacao_didatica',
    'Energia e estequiometria são relações quantitativas; a equação, o diagrama de entalpia e a leitura dos coeficientes são mais claros em 2D.',
    'T1',
    '["A animação percorre estados inicial e final; não representa o mecanismo radicalar real.","A energia de ativação não é representada porque o recorte usa apenas dados termodinâmicos de entalpia.","Água líquida é o estado de referência do ΔH exibido."]'::jsonb,
    '["Balanceamento e conservação de átomos.","Cálculo de ΔH a partir de entalpias de formação.","Interpretação de reações exotérmicas e endotérmicas."]'::jsonb,
    '[
      {"question":"A combustão completa de 1 mol de CH₄ libera cerca de 890 kJ. O que o sinal negativo de ΔH indica?","options":["Que a reação libera energia para a vizinhança.","Que a reação absorve energia da vizinhança.","Que a massa dos produtos é menor.","Que a reação não precisa de energia de ativação."],"correctIndex":0,"explanation":"ΔH negativo significa que o sistema perde entalpia; em pressão constante, essa energia é liberada como calor."},
      {"question":"Por que a equação balanceada usa 2 O₂ para cada CH₄?","options":["Para conservar os quatro átomos de oxigênio necessários aos produtos.","Porque o oxigênio sempre tem coeficiente 2.","Para aumentar a energia liberada sem alterar os produtos.","Porque a água deve ser formada na proporção 1:1."],"correctIndex":0,"explanation":"Cada CH₄ produz um CO₂ (2 O) e dois H₂O (2 O), totalizando 4 O; por isso são necessários 2 O₂."}
    ]'::jsonb
),
(
    gen_random_uuid(),
    'equilibrio-n2o4',
    'quimica',
    '2d',
    'Equilíbrio N₂O₄ ⇌ 2NO₂',
    'Varie concentrações, acompanhe Q e K e veja o sistema se deslocar até o equilíbrio.',
    '["Distinguir Q de K e prever o sentido do deslocamento","Aplicar o princípio de Le Chatelier a perturbações de concentração","Interpretar gráficos de concentração em função do tempo"]'::jsonb,
    'intermediario',
    '[
      {"id":"n2o4Inicial","label":"N₂O₄ inicial","kind":"slider","min":0.01,"max":0.10,"step":0.005,"default":0.05,"unit":"mol/L"},
      {"id":"no2Inicial","label":"NO₂ inicial","kind":"slider","min":0,"max":0.05,"step":0.005,"default":0,"unit":"mol/L"}
    ]'::jsonb,
    '["Achar que a reação para quando atinge o equilíbrio","Inverter Q e K ao comparar","Ignorar que a constante K depende da temperatura"]'::jsonb,
    '["Atkins & de Paula, Physical Chemistry, 10th ed.: Kc ≈ 5,9 × 10⁻³ para N₂O₄(g) ⇌ 2NO₂(g) a 298 K.","NIST-JANAF Thermochemical Tables: ΔH° ≈ +57,2 kJ/mol para a dissociação.","IUPAC. Quantities, Units and Symbols in Physical Chemistry, 3rd ed."]'::jsonb,
    'equilibrium-2d',
    'simulacao',
    'A evolução de concentrações e a comparação Q × K são quantitativas; o gráfico 2D e os valores numéricos comunicam melhor do que uma geometria espacial.',
    'T1',
    '["Cinética simplificada com leis de velocidade elementares e constantes escolhidas para dar uma evolução legível; o equilíbrio é calculado pela termodinâmica.","Solução ideal, volume constante e temperatura fixa em 298 K.","Não há variação de pressão total nem gases não ideais."]'::jsonb,
    '["Reconhecer que Q < K desloca para produtos e Q > K desloca para reagentes.","Aplicar Le Chatelier a adição/remoção de espécies.","Relacionar Kc e Kp para variações de número de mols gasosos."]'::jsonb,
    '[
      {"question":"No equilíbrio N₂O₄(g) ⇌ 2NO₂(g), se Q < K, o que acontece?","options":["A reação avança no sentido dos produtos até Q = K.","A reação avança no sentido dos reagentes até Q = K.","O equilíbrio não se altera.","K diminui até ficar igual a Q."],"correctIndex":0,"explanation":"Q < K indica que há produtos de menos em relação ao equilíbrio; o sistema avança para a direita."},
      {"question":"Adicionar NO₂ a um sistema em equilíbrio desloca a reação para qual lado?","options":["Para os reagentes, consumindo parte do NO₂ adicionado.","Para os produtos, formando mais NO₂.","Não desloca, pois K é constante.","Para os dois lados ao mesmo tempo, sem mudança de concentrações."],"correctIndex":0,"explanation":"O aumento de [NO₂] eleva Q; para restabelecer Q = K, o sistema consome NO₂ e forma N₂O₄."}
    ]'::jsonb
),
(
    gen_random_uuid(),
    'titulacao-acido-base',
    'quimica',
    '2d',
    'Titulação ácido-base',
    'Compare ácido forte e fraco, acompanhe a curva de pH e identifique o ponto de equivalência.',
    '["Interpretar curvas de titulação e localizar o ponto de equivalência","Relacionar a força do ácido ao pH no ponto de equivalência","Escolher um indicador adequado para uma titulação"]'::jsonb,
    'intermediario',
    '[
      {"id":"analito","label":"Analito","kind":"select","default":"HCl (forte)","options":["HCl (forte)","CH₃COOH (fraco)"]},
      {"id":"concentracaoAnalito","label":"Concentração do analito","kind":"slider","min":0.01,"max":0.20,"step":0.01,"default":0.10,"unit":"mol/L"},
      {"id":"volumeAnalito","label":"Volume do analito","kind":"slider","min":10,"max":50,"step":1,"default":25,"unit":"mL"},
      {"id":"concentracaoTitulante","label":"Concentração do titulante","kind":"slider","min":0.01,"max":0.20,"step":0.01,"default":0.10,"unit":"mol/L"},
      {"id":"volumeAdicionado","label":"Volume de titulante adicionado","kind":"slider","min":0,"max":60,"step":0.5,"default":0,"unit":"mL"},
      {"id":"indicador","label":"Indicador","kind":"select","default":"Fenolftaleína","options":["Fenolftaleína","Alaranjado de metila","Azul de bromotimol"]}
    ]'::jsonb,
    '["Achar que o ponto de equivalência de ácido fraco com base forte tem pH 7","Confundir ponto de equivalência com ponto final do indicador","Usar a concentração do titulante no lugar da quantidade de matéria"]'::jsonb,
    '["CRC Handbook of Chemistry and Physics, 84th ed.: Ka(CH₃COOH) = 1,75 × 10⁻⁵ a 25 °C.","CRC Handbook of Chemistry and Physics: faixas de viragem de indicadores ácido-base.","IUPAC. Quantities, Units and Symbols in Physical Chemistry, 3rd ed."]'::jsonb,
    'titration-2d',
    'simulacao',
    'A curva de pH em função do volume é uma relação quantitativa; o gráfico 2D com leitura de eixos é a representação mais direta.',
    'T2',
    '["Solução ideal, eletrólitos totalmente dissociados e atividade igual à concentração.","Temperatura fixa em 25 °C, com Kw = 1,0 × 10⁻¹⁴.","A curva é calculada para ácido monoprótico; não representa ácidos polipróticos."]'::jsonb,
    '["Relacionar o volume de equivalência à estequiometria 1:1.","Interpretar o pH no ponto de equivalência para ácido forte e fraco.","Escolher indicadores cuja faixa de viragem contenha o pH de equivalência."]'::jsonb,
    '[
      {"question":"Na titulação de HCl com NaOH, qual é o pH no ponto de equivalência a 25 °C?","options":["Aproximadamente 7.","Sempre menor que 1.","Sempre maior que 10.","Igual ao pKa do HCl."],"correctIndex":0,"explanation":"HCl e NaOH são eletrólitos fortes; no ponto de equivalência resta apenas NaCl dissolvido, e a água neutra tem pH ≈ 7 a 25 °C."},
      {"question":"Por que a fenolftaleína é mais adequada para titular ácido acético com NaOH do que alaranjado de metila?","options":["Porque o pH de equivalência é básico e cai na faixa de viragem da fenolftaleína.","Porque a fenolftaleína reage com o ácido acético.","Porque o alaranjado de metila não muda de cor em soluções ácidas.","Porque a fenolftaleína mantém o pH constante."],"correctIndex":0,"explanation":"A hidrólise do acetato deixa o pH de equivalência acima de 7; a fenolftaleína vira na faixa básica, enquanto o alaranjado vira em pH ácido."}
    ]'::jsonb
);

INSERT INTO visualization_topics (visualization_id, topic_id)
SELECT v.id, t.id
FROM visualizations v
JOIN enem_topics t ON t.subject_code = v.subject_code
WHERE v.slug = 'combustao-metano' AND t.slug IN ('termoquimica', 'combustao');

INSERT INTO visualization_topics (visualization_id, topic_id)
SELECT v.id, t.id
FROM visualizations v
JOIN enem_topics t ON t.subject_code = v.subject_code
WHERE v.slug = 'equilibrio-n2o4' AND t.slug IN ('equilibrio-quimico', 'n2o4-no2');

INSERT INTO visualization_topics (visualization_id, topic_id)
SELECT v.id, t.id
FROM visualizations v
JOIN enem_topics t ON t.subject_code = v.subject_code
WHERE v.slug = 'titulacao-acido-base' AND t.slug IN ('equilibrio-ionico', 'titulacao');
