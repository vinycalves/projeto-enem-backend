# ENEM Aberto — Backend

API em **Rust + Axum + PostgreSQL** para uma plataforma gratuita de estudos para o ENEM.

O projeto é open source, sem anúncios, sem IA e sem cobrança. O conteúdo inicial é autoral e licenciado em **CC BY-SA 4.0**; o código é **MIT**.

## O que já está implementado

- Cadastro e login com **e-mail e senha**, sem confirmação de e-mail.
- Sessão em cookie `HttpOnly` com token opaco armazenado como hash no banco.
- Currículo por área, disciplina e conceito.
- Questões com cinco alternativas, tentativa, correção e explicação.
- **Discussão da questão após a tentativa**: comentários gerais ou por alternativa.
- Gabarito e explicação só são liberados depois de uma tentativa (endpoint protegido).
- **Importador de questões oficiais** do ENEM (2009–2023) via API do enem.dev, com imagens, idioma, fonte e licença.
- Estatísticas básicas do estudante e últimos estudos.
- Calendário oficial do ENEM 2026, com datas confirmadas e itens "a definir".
- Endpoint de doação Pix; a chave fica apenas em variável de ambiente do backend.
- Testes unitários e testes de integração com PostgreSQL real.
- Dockerfile e Docker Compose para execução local (Compose somente com PostgreSQL).

## Stack

| Camada | Tecnologia |
|---|---|
| Linguagem | Rust 1.99, edition 2024 |
| HTTP | Axum 0.8 |
| Banco | PostgreSQL 18 |
| Acesso a dados | SQLx 0.9 |
| Senhas | Argon2id |
| Sessão | Cookie HttpOnly + token opaco com hash SHA-256 |
| Testes | `cargo test` + `sqlx::test` |

## Rodar localmente

O `docker-compose.yml` sobe **apenas o PostgreSQL**. Backend e frontend rodam diretamente na máquina:

```bash
cp .env.example .env
# edite o .env e preencha PIX_KEY, se quiser habilitar a doação
docker compose up -d
```

Depois, em outro terminal:

```bash
cargo run
```

- API local: `http://localhost:8081` (configurável em `BIND_ADDR`)
- PostgreSQL: `localhost:5432`

> A porta `8080` desta máquina pode estar ocupada por outro serviço. O `.env` de exemplo usa `8081` para evitar conflito; o frontend faz proxy para a porta definida no `proxy.conf.json`. Em produção, o container usa `8080` internamente.

As migrações são aplicadas automaticamente na inicialização.

### Imagens Docker

Os Dockerfiles continuam no projeto para build independente das imagens de backend e frontend. O Compose **não** sobe esses serviços; ele é apenas o banco local.

## Importar questões oficiais

O binário `import-enem` baixa as provas disponíveis na API pública do [enem.dev](https://enem.dev) e grava no PostgreSQL de forma idempotente. Ele cria, por área e por ano, uma disciplina “Questões oficiais” e um conceito “ENEM {ano}”.

```bash
# importa 2009–2023
cargo run --bin import-enem -- --from-year 2009 --to-year 2023

# teste rápido, sem gravar
cargo run --bin import-enem -- --from-year 2023 --to-year 2023 --limit 5 --dry-run

# um ano específico
cargo run --bin import-enem -- --from-year 2023 --to-year 2023
```

O importador:

- usa `limit=50` e paginação, respeitando o limite da API;
- tenta novamente em caso de `429`;
- deduplica por `external_id`;
- atualiza a questão e recria os assets em reimportações;
- mantém `source` e `license` visíveis na questão;
- registra imagens de contexto e de alternativas em `question_assets`.

> **Fonte e direitos:** os dados são obtidos do enem.dev, que se descreve como projeto comunitário e declara os dados como públicos. O projeto mantém a atribuição e a fonte no banco. Antes de usar comercialmente ou em maior escala, confira os termos da fonte e dos direitos de textos/imagens de terceiros.

## Testes

Com um PostgreSQL acessível:

```bash
DATABASE_URL=postgres://enem:enem@localhost:5432/enem cargo test
```

O `sqlx::test` cria bancos temporários e aplica as migrações. Os testes cobrem:

- cadastro, login, sessão, logout e e-mail duplicado;
- autorização para resposta, discussão e comentário;
- correção de tentativa sem vazamento de gabarito;
- comentários e denúncias;
- calendário oficial;
- endpoint Pix;
- seed do currículo.

Verificações de qualidade:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Variáveis de ambiente

| Variável | Obrigatória | Padrão | Descrição |
|---|---|---|---|
| `DATABASE_URL` | Sim | — | URL do PostgreSQL |
| `BIND_ADDR` | Não | `0.0.0.0:8080` | Endereço da API |
| `CORS_ORIGIN` | Não | `http://localhost:4200` | Origem permitida no CORS |
| `COOKIE_SECURE` | Não | `false` | Use `true` em produção com HTTPS |
| `SESSION_TTL_DAYS` | Não | `30` | Duração da sessão |
| `PIX_KEY` | Não | — | Chave Pix exposta apenas pelo endpoint de doação |

> A chave Pix **não** é embutida no frontend. O botão de apoio só a busca quando o usuário clica.

## Endpoints principais

Base: `/api/v1`

| Método | Rota | Descrição |
|---|---|---|
| `GET` | `/health` | Healthcheck |
| `POST` | `/auth/register` | Cadastro com `{email, password}` |
| `POST` | `/auth/login` | Login |
| `POST` | `/auth/logout` | Encerra a sessão |
| `GET` | `/auth/me` | Usuário da sessão |
| `GET` | `/curriculum/areas` | Áreas, disciplinas e contagens |
| `GET` | `/curriculum/subjects/{id}/concepts` | Conceitos da disciplina |
| `GET` | `/curriculum/concepts/{id}` | Conceito e explicação |
| `GET` | `/questions?concept_id={id}` | Questões do conceito |
| `GET` | `/questions/{id}` | Questão sem gabarito |
| `POST` | `/questions/{id}/attempts` | Tentativa `{selected_option}` |
| `GET` | `/questions/{id}/discussion` | Discussão (exige tentativa) |
| `POST` | `/questions/{id}/comments` | Comentário (exige tentativa) |
| `POST` | `/comments/{id}/report` | Denúncia de comentário |
| `GET` | `/questions/{id}/answer` | Gabarito e explicação (exige tentativa) |
| `GET` | `/me/stats` | Estatísticas e últimos estudos |
| `GET` | `/enem/calendar` | Calendário oficial do ENEM |
| `GET` | `/support/pix` | Chave Pix, se configurada |

Erros seguem o formato:

```json
{
  "error": {
    "code": "bad_request",
    "message": "Informe um e-mail válido."
  }
}
```

## Estrutura

```text
src/
  auth.rs          # cadastro, login, sessão e extractor de usuário
  calendar.rs      # calendário oficial
  config.rs        # configuração por ambiente
  curriculum.rs    # áreas, disciplinas e conceitos
  error.rs         # erros HTTP
  importer.rs      # importação idempotente via enem.dev
  questions.rs     # questões, tentativas, discussão, resposta e estatísticas
  support.rs       # Pix
  validation.rs    # validação e normalização
  lib.rs           # router
  main.rs          # bootstrap
  bin/import-enem.rs
migrations/
  0001_initial.sql
  0002_seed.sql
  0003_official_questions.sql
  0004_external_id_constraint.sql
tests/
  api.rs
```

## Decisões e limites atuais

- **Sem IA.** Não há tutor, RAG ou geração de conteúdo em tempo real.
- **Sem observabilidade externa.** Apenas logs básicos de aplicação.
- **Sem backups automáticos.** O servidor/banco deve ter a política definida pelo operador.
- **Sem painel administrativo.** O conteúdo inicial é carregado por migração. A curadoria pode ser adicionada depois.
- **Sem confirmação de e-mail.** O cadastro é imediato, conforme definido para esta versão.
- **Sem pagamentos.** A doação é opcional e via Pix.

## Currículo

A taxonomia de trabalho, a estrutura das quatro áreas e a situação de cobertura estão em [`docs/curriculum.md`](docs/curriculum.md).

## Licença

Código: MIT. Conteúdo autoral de exemplo: CC BY-SA 4.0. Questões oficiais: fonte e licença registradas por item; consulte a fonte original.
