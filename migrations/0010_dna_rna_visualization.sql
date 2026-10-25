UPDATE visualizations
SET title = 'DNA e RNA em 3D',
    summary = 'Explore a dupla hélice do DNA e a fita simples do RNA, com bases, pareamento e controles de câmera.',
    learning_objectives = '["Reconhecer a estrutura de dupla hélice do DNA","Distinguir DNA de RNA e identificar as bases U, C, G e A no RNA","Relacionar a estrutura dos ácidos nucleicos à replicação e à transcrição"]'::jsonb,
    params = '[
      {"id":"molecule","label":"Molécula","kind":"select","default":"DNA","options":["DNA","RNA"]},
      {"id":"showBases","label":"Mostrar bases nitrogenadas","kind":"toggle","default":true},
      {"id":"highlight","label":"Destacar bases","kind":"select","default":"A (A–T/A–U)","options":["A (A–T/A–U)","C (C–G)"]},
      {"id":"opening","label":"Abertura visual da hélice (modo DNA)","kind":"slider","min":0,"max":1,"step":0.05,"default":0.1}
    ]'::jsonb,
    common_mistakes = '["Achar que o RNA é sempre uma fita dupla","Trocar uracila por timina no RNA","Concluir que a abertura visual da hélice representa o mecanismo real de replicação"]'::jsonb,
    sources = '["Watson, J. D.; Crick, F. H. C. Molecular Structure of Nucleic Acids: A Structure for Deoxyribose Nucleic Acid. Nature, 1953.","NCBI Bookshelf: Molecular Biology of the Cell, capítulos sobre estrutura de DNA e transcrição.","Modelo didático esquemático; sequências exibidas são arbitrárias."]'::jsonb,
    quiz = '[
      {"question":"Qual base nitrogenada aparece no RNA e não no DNA?","options":["Adenina","Timina","Uracila","Guanina"],"correctIndex":2,"explanation":"O RNA usa uracila no lugar da timina; adenina, citosina e guanina aparecem nos dois ácidos nucleicos."},
      {"question":"O que a dupla hélice do DNA representa?","options":["Duas fitas antiparalelas pareadas por bases.","Uma única fita sem pareamento.","Uma proteína com quatro cadeias.","Um carboidrato de reserva."],"correctIndex":0,"explanation":"O DNA é formado por duas fitas antiparalelas unidas por pareamento de bases A–T e C–G."}
    ]'::jsonb,
    updated_at = now()
WHERE slug = 'dna-3d';
