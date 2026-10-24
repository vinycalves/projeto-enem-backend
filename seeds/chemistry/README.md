# Seeds de Química

## elements.json

118 elementos químicos com número atômico, símbolo, nome em pt-BR, massa atômica,
grupo, período, eletronegatividade de Pauling, raio atômico, configuração
eletrônica, categoria, estado padrão e cor CPK.

Fontes e licenças:

- Bowserinator/Periodic-Table-JSON: dados de número, massa, período, grupo,
  categoria e configuração eletrônica. https://github.com/Bowserinator/Periodic-Table-JSON
- AlexGustafsson/molecular-data: `json/elements.json` e
  `json/elementsLocale.json` (tradução pt e propriedades complementares).
  https://github.com/AlexGustafsson/molecular-data
  O projeto declara dados derivados de Chris Andrejewski (2015), com licença
  permissiva que exige a manutenção do aviso de copyright e permissão.
- IUPAC/CIAAW: referência para pesos atômicos padrão; os valores foram usados
  conforme publicados nos datasets acima, sem interpolação.

O arquivo é gerado offline; nenhum download acontece em runtime. Nomes em
pt-BR dos elementos 113, 115, 117, 118, 110 e 111 foram normalizados para a
forma usual em português. Categorias "unknown..." foram rotuladas como
"Categoria prevista" para não apresentar como definitivo um dado previsto.

## reactions.json

Reações curadas para os recortes iniciais. Cada reação passa pelo validador Rust
(conservação de átomos e carga, coeficientes mínimos, atomMapping bijetivo,
composição compatível com a fórmula e fontes obrigatórias).

## datasets.json

Constantes termodinâmicas e ácido-base com condições e fontes declaradas.
