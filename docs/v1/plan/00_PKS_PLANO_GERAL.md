# PKS - Plano de Desenvolvimento Geral

> Plano abrangente para implementar o PRD do Prometheus Knowledge System.
> Gerado em: 2026-03-07

---

## Scout Summary

| Item | Detalhe |
|------|---------|
| **Linguagem principal** | Rust |
| **Busca textual** | Tantivy (BM25) |
| **Embeddings** | Ollama (`nomic-embed-text`) — pos-MVP (Fase C M5) |
| **Serialização** | bincode (snapshots segmentados) |
| **Journal/Durabilidade** | Git como append-only log |
| **Protocolo AI** | MCP via stdio (`pks --stdio`) |
| **Concorrência** | Single-thread NYC-style (zero locks) |
| **Repo atual** | Documentação/QA (Markdown + Notion MCP) |

---

## Requisitos de Sistema

Conforme PRD (secao "Requisitos de Sistema"):

| Requisito | Minimo | Recomendado |
|---|---|---|
| **Sistema Operacional** | macOS 13+ (Ventura) / Linux kernel 5.15+ | macOS 14+ (Sonoma, Apple Silicon) |
| **RAM** | 8 GB | 16 GB+ |
| **Armazenamento** | SSD (HDD nao suportado) | NVMe |
| **Ollama** | v0.3+ com `nomic-embed-text` (~275MB VRAM) — apenas pos-MVP (M5) | GPU dedicada ou Apple Neural Engine |
| **Git** | 2.30+ | 2.40+ (melhorias em worktree) |
| **Git LFS** | 3.0+ (apenas para M6 — sincronizacao de snapshots vetoriais) | 3.4+ |
| **Rust** | 1.75+ (apenas para build do Daemon) | Stable mais recente |

> **Nota MVP:** No MVP (BM25-only), Ollama nao e necessario. O Daemon funciona 100% sem Ollama instalado.
> **Nota RAM:** Com BM25-only, consumo de RAM e irrisorio (dezenas de KB por 1000 notas). Watermarks (`PKS_MAX_VECTORS`) relevantes apenas pos-M5 (Embeddings).

---

## Estrategia de Autenticacao (`.env`)

Toda autenticacao do PKS e dos MCPs conectados usa **variaveis de ambiente** como mecanismo primario, carregadas de um arquivo `.env` na raiz do projeto. Alternativamente, credenciais podem ser lidas do **keychain do SO** (`security` no macOS, `secret-tool` no Linux) conforme PRD F2.12. Sem config files complexos, sem secrets no Git.

### Fluxo

```
.env.example  (versionado, valores vazios, documentacao)
     │
     │  cp .env.example .env
     ▼
.env          (NAO versionado, .gitignore ja cobre)
     │
     │  Carregado por:
     ├── PKS Daemon (crate `dotenvy` no Rust)
     ├── MCP Servers (campo "env" do .mcp.json)
     └── Claude Code (env vars do shell)
```

### Arquivos

| Arquivo | Versionado? | Conteudo |
|---------|-------------|----------|
| `.env.example` | Sim | Template com todas as vars, valores vazios, comentarios |
| `.env` | Nao (`.gitignore`) | Valores reais das credenciais |
| `.mcp.json` | Sim | Referencia `${VAR_NAME}` para injetar env vars nos MCP servers |

### Variaveis por Servico

| Variavel | Servico | Obrigatoria? | Fase |
|----------|---------|--------------|------|
| `NOTION_TOKEN` | Notion MCP (import/export) | Sim | C (M7) |
| `PKS_PORT` | ~~PKS Daemon MCP SSE~~ — **Obsoleto** (transporte mudou para stdio; sem porta de rede) | Removida | A (M1) |
| `PKS_VAULTS_DIR` | Diretorio raiz dos vaults | Nao (default: ~/pks-vaults) | A (M1) |
| `OLLAMA_BASE_URL` | Ollama embeddings | Nao (default: localhost:11434) | C (M5) |
| `OLLAMA_EMBED_MODEL` | Modelo de embedding | Nao (default: nomic-embed-text) | C (M5) |
| `PKS_EMBEDDING_PROVIDER` | Provider de embeddings | Nao (default: ollama) | C (M5) |
| `JIRA_BASE_URL` | Jira (futuro) | Nao | pos-Fase C |
| `JIRA_EMAIL` | Jira (futuro) | Nao | pos-Fase C |
| `JIRA_API_TOKEN` | Jira (futuro) | Nao | pos-Fase C |
| `LINEAR_API_KEY` | Linear (futuro) | Nao | pos-Fase C |
| `GITHUB_TOKEN` | GitHub repos remotos | Nao | B (M4) |
| `PKS_REMOTE_POLL_INTERVAL` | Intervalo de fetch para repos remotos | Nao (default: 300s) | B (M4) |
| `PKS_THROTTLE_MS` | Tuning do pipeline | Nao (default: 200) | A (M2) |
| `PKS_FILA1_MAX` | Limite Fila 1 | Nao (default: 1000) | A (M2) |
| `PKS_FILA2_MAX` | Limite Fila 2 | Nao (default: 500) | A (M2) |
| `PKS_MAX_VECTORS` | Teto de vetores em RAM | Nao (default: 500000) | C (M5) |
| `PKS_HIBERNATE_DAYS` | Dias para hibernacao | Nao (default: 7) | C (M5) |
| `PKS_CHUNK_MAX_TOKENS` | Tamanho maximo do chunk | Nao (default: 400) | A (M1) |
| `PKS_CHUNK_OVERLAP_TOKENS` | Overlap entre chunks sliding window | Nao (default: 80) | A (M1) |
| `PKS_CHUNK_MIN_TOKENS` | Tamanho minimo antes de agrupar | Nao (default: 100) | A (M1) |
| `PKS_LOG_MAX_SIZE` | Tamanho maximo de log antes de rotacao | Nao (default: 50MB) | C (M6) |
| `PKS_SNAPSHOT_INTERVAL_COMMITS` | Commits entre snapshots | Nao (default: 100) | A (M3) |
| `PKS_SNAPSHOT_INTERVAL_SECS` | Segundos de inatividade para snapshot | Nao (default: 300) | A (M3) |
| `PKS_DEBOUNCE_WINDOW_MS` | Janela de debounce para deteccao | Nao (default: 500) | B (M4) |
| `PKS_IMPORT_MAX_SIZE` | Tamanho maximo de conteudo importado | Nao (default: 1MB) | C (M7) |
| `PKS_BACKUP_COMPRESS` | Compressao zstd no backup | Nao (default: false) | C (M6) |
| `PKS_VECTOR_REMOTE_URL` | URL do repo Git LFS satélite exclusivo deste projeto para sincronizacao de snapshots vetoriais (Modelo 1:1) | Nao | C (M6) |
| `PKS_PARSE_TIMEOUT_MS` | Timeout maximo do parser Markdown por arquivo | Nao (default: 5000) | A (M1) |
| `PKS_PARSE_MAX_NESTING` | Limite de aninhamento sintatico (ReDoS) | Nao (default: 100) | A (M1) |

### Como o `.mcp.json` usa as env vars

```json
{
  "notion": {
    "env": { "NOTION_TOKEN": "${NOTION_TOKEN}" }
  },
  "pks": {
    "type": "stdio",
    "command": "/path/to/WellzestaNotion/pks/target/release/pks",
    "args": ["--stdio"],
    "env": {
      "PKS_VAULTS_DIR": "/Users/<user>/VSCodeProjects"
    }
  }
}
```

O Claude Code resolve `${NOTION_TOKEN}` do ambiente do shell (que carrega do `.env`).

### Como o PKS Daemon carrega no Rust

```rust
// Cargo.toml: dotenvy = "0.15"
fn main() {
    dotenvy::dotenv().ok(); // carrega .env se existir
    let port = std::env::var("PKS_PORT").unwrap_or("3030".into());
    let vaults = std::env::var("PKS_VAULTS_DIR").unwrap_or("~/pks-vaults".into());
}
```

### Crate adicional no Cargo.toml

```toml
dotenvy = "0.15"  # Carregamento de .env
```

### Seguranca

- `.env` ja esta no `.gitignore` (linhas 13-14)
- `.env.example` versionado como documentacao viva
- Nenhuma credencial hardcoded no codigo
- Para ambientes de CI/CD: env vars injetadas pelo runner (GitHub Actions secrets, etc.)

### Artefatos em Disco

| Artefato | Path | Descricao |
|----------|------|-----------|
| Divida vetorial | `~/.pks/embedding_debt.jsonl` | Chunks pendentes de embedding |
| Logs | `~/.pks/logs/` | Logs estruturados JSON com rotacao |
| Snapshots | `{PKS_VAULTS_DIR}/<repo>/snapshots/<repo_id>.bin` | Snapshots bincode segmentados |
| Drain Fila 1 | `~/.pks/fila1_drain.jsonl` | Fila 1 serializada em graceful shutdown (T6.4) |
| Drain Sync Queue | `~/.pks/sync_queue_drain.jsonl` | Tracker Sync Queue serializada em graceful shutdown (T6.4 — presente apenas quando T7.3 estiver ativo) |

---

## MCPs Existentes Reutilizaveis (Evitar Reinventar a Roda)

| MCP Server | Repo/Link | O que aproveitar no PKS |
|------------|-----------|------------------------|
| **knowledge-rag** | [lyonzin/knowledge-rag](https://github.com/lyonzin/knowledge-rag) | Hybrid search (BM25 + Semantic) com Reciprocal Rank Fusion — arquitetura mais alinhada ao PKS |
| **rust-local-rag** | [ksaritek/rust-local-rag](https://github.com/ksaritek/rust-local-rag) | Referencia de MCP server em Rust com Ollama embeddings locais, usa `rmcp` SDK |
| **rmcp (Rust MCP SDK)** | [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) | SDK oficial Rust para MCP — base do servidor stdio do PKS |
| **obsidian-mcp-server** | [cyanheads/obsidian-mcp-server](https://github.com/cyanheads/obsidian-mcp-server) | Referencia de interacao com Obsidian vaults (leitura, busca, frontmatter YAML) |
| **obsidian-mcp-tools** | [jacksteamdev/obsidian-mcp-tools](https://github.com/jacksteamdev/obsidian-mcp-tools) | Semantic search em Obsidian + templates — patterns de chunking e indexacao |
| **rust-mcp-filesystem** | [rust-mcp-stack/rust-mcp-filesystem](https://github.com/rust-mcp-stack/rust-mcp-filesystem) | Async filesystem ops em Rust via MCP — patterns de I/O seguro e performatico |
| **server-git** | [modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers) | MCP server de referencia para operacoes Git (diff, log, status) |
| **server-memory** | [modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers) | Knowledge graph persistente — padrao de memoria entre sessoes |
| **Qdrant MCP** | [qdrant/mcp-server-qdrant](https://github.com/qdrant/mcp-server-qdrant) | Referencia de vector DB com MCP (caso PKS migre para HNSW no futuro) |

### Recomendacao de Uso

1. **Clonar e estudar `knowledge-rag`** como referencia primaria — e o projeto mais proximo da arquitetura PKS (hybrid BM25 + semantic, local, MCP)
2. **Usar `rmcp` SDK** como dependencia direta no `Cargo.toml` do PKS
3. **Estudar `rust-local-rag`** para patterns de integracao Rust + Ollama via MCP
4. **Nao usar** obsidian-mcp-server diretamente (depende de REST API plugin do Obsidian) — apenas como referencia de design

---

## Estrutura de Fases e Milestones

```
Fase A: Motor Prevalente de RAG (M1 + M2 + M3)
  ├── M1: Engine Rustica (MCP stdio, Chunking MD, BM25, FS-as-Config)
  ├── M2: Pipeline NYC-Style BM25-only (Double-Buffered, Debounce)
  └── M3: Persistencia e Resiliencia (Event Sourcing, Gestao Manual, Snapshots)

Fase B: Git Journaling & Vaults (M4)
  └── M4: Conexao com Hospedeiro (Vault structure, Branch pks-knowledge, Hooks, pks doctor, Reindex, Remote Repos)

Fase C: Embeddings, Operação & Trackers (M5 + M6 + M7)
  ├── M5: Embeddings Vetoriais (Ollama, Busca Hibrida, LRU/Hibernacao)
  ├── M6: Maturacao e Operacao (Observabilidade, Contingencia LFS)
  └── M7: Trackers e Expansao (Import/Export Notion, Sync Queue)
```

---

## Arquivos do Plano

| Arquivo | Conteudo |
|---------|----------|
| `01_FASE_A_motor_prevalente.md` | Tasks M1-M3: Engine, Pipeline, Persistencia |
| `02_FASE_B_git_journaling.md` | Tasks M4: Vaults, Git worktree, Hooks, Remote Repos |
| `03_FASE_C_trackers_operacao.md` | Tasks M5-M7: Embeddings, Import/Export, Observabilidade |

---

## Dependencias entre Fases

```
Fase A (Motor) ──obrigatoria──▶ Fase B (Git Journaling)
                                      │
Fase A (Motor) ──obrigatoria──▶ Fase C (Embeddings, Trackers & Ops)
                                      │
                          Fase B ──parcial──▶ Fase C (M6/M7)
                          (F1.2 branch necessario para F1.3/F1.4)
```

**Fase A e obrigatoria antes de B e C.** M5 (Embeddings) pode iniciar apos Fase A; M7 (Trackers) requer Fase B parcial. Fases B e C podem ter paralelismo parcial apos M4 estar completo.

**Cenarios de falha por fase:** Fase A (T3.4) cobre falhas de engine e replay. Fase B (T4.5) cobre falhas Git e worktree. Fase C (M5) cobre falhas de Ollama (5 estados de degradacao). Fase C (M6/M7) deve cobrir falhas de tracker API (timeout, auth expirado) e backup (git-lfs nao instalado, cota LFS estourada).

### Notas de Reconciliacao com o Roteiro do PRD

> **Nota geral sobre reorganizacao de milestones:** O Plano reorganizou TODOS os milestones em relacao a secao "Criterios de Aceite" do PRD. O mapeamento completo e:
> - **PRD M1 Criterios** → Plano M1 (majoritariamente), exceto `pks status` que foi movido para M6 (T6.3).
> - **PRD M2 Criterios** (Git journaling, `pks init`, `pks doctor`) → Plano M4 (Fase B — Git Journaling & Vaults).
> - **PRD M3 Criterios** (Daemon como servico de SO, pipeline double-buffered, logs, health check, metricas, localhost-only, `pks validate`) → Plano M2 (pipeline double-buffered) + M6 (servico SO, logs estruturados, health check, metricas, localhost binding, `pks validate`).
> - **PRD M4 Criterios** (persistencia, snapshots bincode, vector clock, reindex) → Plano M3 (Persistencia e Resiliencia).
>
> A secao "Roteiro" do PRD e mais alinhada ao Plano do que a secao "Criterios de Aceite". As entradas individuais abaixo detalham divergencias pontuais.

| Divergencia | PRD Roteiro | Plano | Justificativa |
|-------------|-------------|-------|---------------|
| **F1.5** | Atribuida a M2 ("Filtros e Rotacao de vetores") | Movida **inteiramente** para M7 (T7.5) como "Armazenamento Seletivo" | No PRD, F1.5 trata de *o que vai ao prometheus/* — decisao de filtragem de conteudo. Essa logica so faz sentido operacionalmente quando trackers estao integrados (M7). F1.5 NAO esta parcialmente em M2 — foi movida por completo para M7. A parte de "Rotacao de vetores" mencionada no PRD Roteiro M2 e coberta por T5.4e (LRU/Hibernacao) em M5, que e uma feature distinta (F2.10). |
| **F2.6** | Nao atribuida explicitamente a nenhum milestone no Roteiro | Adicionada a M4 (T4.7) | F2.6 (Memoria Reflexiva Global) depende de `pks-knowledge` branch (F1.2/T4.2) e do vault structure (F1.1/T4.1), ambos em M4. Colocacao natural junto ao Git Journaling. |
| **F2.7** | Nao atribuida explicitamente a nenhum milestone no Roteiro | Adicionada a M4 (T4.6) | F2.7 (Indexacao Multi-Repo Distribuida — repos remotos) depende de repo_watcher (T1.4/M1) e git_journal (T4.4/M4). Repos remotos sao clonados em `~/pks-vaults/` e indexados pelo mesmo pipeline. Colocacao em M4 agrupa toda logica de integracao Git. |
| **F2.2** | Nao listada como divergencia | Distribuida entre M1 (T1.5) e M6 (T6.4) | Daemon e MCP stdio iniciam em M1; graceful shutdown e systemd unit adicionados em M6. |
| **F2.4** | Nao listada como divergencia | Distribuida entre M1 (T1.2) e M5 (T5.3e) | Chunking basico (heading + BM25 dedup) em M1; sliding window + dedup Ollama em M5. |
| **F2.5** | Nao listada como divergencia | Distribuida entre M3 (T3.4) e M4 (T4.5) | Recovery de engine em M3; recovery de Git/worktree em M4. |
| **F2.12** | Nao listada como divergencia | Distribuida entre M1 (T1.2), M6 (T6.6) e M7 (T7.4) | Sanitizacao basica em M1; seguranca MCP em M6; sanitizacao de conteudo importado em M7. |
| **Fase 1 PRD** | Criterios de aceite da "Fase 1" do PRD | Satisfeita por M4 (F1.1, F1.2) + M7 (F1.3-F1.6) | A "Fase 1 do PRD" e declarada completa quando M4 + M7 estao concluidos. |
| **M5 (Embeddings) no PRD vs Plano** | PRD: M5 = Embeddings (Fase 2) | Plano: M5 = Embeddings (Fase C, pos-MVP) | Alinhado com PRD: Embeddings sao pos-MVP. Decisao STEERING 3.5 (Opcao A) moveu T2.1/T2.3/T2.4 de Fase A M2 para Fase C M5. |
| **F2.10** | Gestao de Memoria | T3.3 (manual, MVP) + T5.4e (LRU automatico, M5) | MVP: gestao manual (Clone=Carregar, Delete=Descarregar). Automatica (LRU/Hibernacao) adiada para M5 com embeddings. Decisao STEERING 3.8 (Opcao A). |
| **`pks status`** | PRD Roteiro M1: "pks status exibe resumo basico" | Movido para M6 (T6.3) | No MVP (Fase A), o daemon e iniciado manualmente e nao ha necessidade de CLI de diagnostico. `pks status` e `pks validate` fazem sentido como parte da Maturacao e Operacao (M6) quando o daemon opera como servico de SO. |
| **M6 Criterio DR** | PRD Criterios de Aceite M6 inclui "Perda total da maquina com cloud: git clone do sub-repo LFS reconstitui snapshots" | Plano: esse criterio pertence a M6 (T6.5 — Disaster Recovery / Sincronizacao LFS) | Disaster Recovery via Git LFS e escopo de M6 (F2.14/T6.5). O criterio coincide com o milestone correto — M6 e Maturacao e Operacao, que inclui T6.5 (Contingencia e Disaster Recovery via Git LFS). |
| **M7 PRD** | PRD Criterios de Aceite: M7 = "Shadow Repositories" | Plano: M7 = Trackers Contextuais (conforme Roteiro do PRD) | O PRD tem inconsistencia interna: o diagrama de Roteiro define M7 como "Trackers Contextuais" mas a secao de Criterios de Aceite rotula M7 como "Shadow Repositories". O plano segue o Roteiro. Shadow Repos sao pos-Fase C (Apendice A). |

---

## Matriz de Rastreabilidade: PRD Feature → Task

| Feature PRD | Descricao | Task(s) | Milestone |
|-------------|-----------|---------|-----------|
| **F1.1** | Estrutura do Vault por Projeto | T4.1 | M4 |
| **F1.2** | Integracao Git (branch `pks-knowledge`, worktree, conflitos) | T4.2, T4.3 | M4 |
| **F1.3** | Import de Tracker Externo | T7.1 | M7 |
| **F1.4** | Export para Tracker Externo | T7.2 | M7 |
| **F1.5** | Armazenamento Seletivo (politica system-wide) | T7.5 | M7 |
| **F1.6** | Tracker Sync Queue | T7.3 | M7 |
| **F2.1** | Transacao Prevalente via Git (hooks, FS Events, debounce) | T4.4 | M4 |
| **F2.2** | Daemon Continuo + Servidor MCP stdio | T1.5, T6.4 | M1, M6 |
| **F2.3** | Double-Buffered Pipeline NYC-Style (BM25-only no MVP) | T2.2 | M2 |
| **F2.4** | Estrategia de Chunking (heading + BM25 dedup no MVP). Nota: T6.3 (`pks validate`) verifica ausencia de tombstones residuais. Sliding window + dedup Ollama em T5.3e (M5). | T1.2, T5.3e | M1, M5 |
| **F2.5** | Tolerancia a Falhas e Recuperacao | T3.4, T4.5 | M3, M4 |
| **F2.6** | Memoria Reflexiva Global | T4.7 | M4 |
| **F2.7** | Indexacao Multi-Repo Distribuida (local + remoto) | T1.4, T4.6 | M1, M4 |
| **F2.8** | Posicionamento CAP (AP) | Criterios AP embutidos em T2.2, T3.4, T4.6. No MVP, AP verificado com BM25-only. | M2, M3, M4 |
| **F2.9** | Event Sourcing / Vector Clock (nota: Vector Clock usa chave `(repo_id, branch)` e rastreia HEADs de TODOS os branches monitorados por repo, incluindo `main`, `feature/*`, `pks-knowledge`) | T3.2 | M3 |
| **F2.10** | Gestao de Memoria (manual no MVP, LRU automatico pos-MVP) | T3.3 (manual, M3) + T5.4e (automatica, M5) | M3, M5 |
| **F2.11** | Filesystem-as-Config | T1.4 | M1 |
| **F2.12** | Seguranca (localhost, ReDoS, sanitizacao, credenciais) | T1.2, T6.6, T7.4 | M1, M6, M7 |
| **F2.13** | Observabilidade (logs, health, CLI diag) | T6.1, T6.2, T6.3 | M6 |
| **F2.14** | Sincronizacao LFS em Repo Satelite / Disaster Recovery | T6.5 | M6 |
| **D1** | Vault autocontido por projeto | T4.1 | M4 |
| **D2** | Modelo nomic-embed-text como default | T5.1e | M5 |
| **D3** | Daemon continuo (nao batch) | T1.5, T6.4 | M1, M6 |
| **D4** | Tool MCP nativa | T1.5 | M1 |
| **D5** | Git History como Journal Prevalente | T3.4 (Git-Journal Replay) | M3 |
| **D6** | stdio como transporte MCP (decisão original era SSE; implementação final adotou stdio) | T1.5 | M1 |
| **D7** | NYC-style single-thread (zero locks) | T2.2 | M2 |
| **D8** | FIFO assincrono com backpressure | T2.2 | M2 |
| **D9** | Rust como linguagem | T1.1 | M1 |
| **D10** | Chunking semantico por heading | T1.2 | M1 |
| **D11** | Localhost-only | T1.5, T6.6 | M1, M6 |
| **D12** | Snapshots bincode segmentados c/ Magic Version Header | T3.1 | M3 |
| **D13** | Branch pks-knowledge + worktree | T4.2, T4.3 | M4 |
| **TDD** | Golden dataset 50 notas + score floors + `cargo test --features integration` | T1.6 | M1 |
| **TDD (Fase C)** | T7.1/T7.2 atualizam `tests/test_mcp_e2e.rs` para `pks_import_tracker` e `pks_export_tracker`. Golden dataset expandido com notas importadas de tracker para busca cross-domain. | T7.1, T7.2 | M7 |
| **F2.13 ext** | Metricas estendidas: `pks_tracker_sync_queue_depth`, `pks_embedding_debt_entries`, `pks_last_commit_indexed` (por repo) | T6.2 | M6 |
| **F2.2 ext** | `list_knowledge_vaults` (listagem de vaults registrados) | T1.5 | M1 |
| **EmbeddingProvider** | Trait `EmbeddingProvider` para abstrato de provider de embeddings (Apendice C secao 4) | T5.1e | M5 |

---

## Criterios de Qualidade Globais

| Criterio | Threshold |
|----------|-----------|
| Cobertura de testes | >= 80% (unitarios Rust) |
| Latencia de query (p95) | < 1ms |
| Golden dataset accuracy | >= 90% top-3 |
| Max linhas por arquivo | 200 |
| Max linhas por funcao | 30 |
| Max complexidade ciclomatica | 10 |
| Zero mocks em testes | Obrigatorio (nota: "Cliente MCP mock" do PRD TDD refere-se a um cliente de teste real conectando ao daemon real via stdio, nao a mock libraries; o termo "mock" no PRD e no sentido de "test client", nao de mock/stub de dependencias internas) |
| Testes de integracao | `cargo test --features integration` (conforme PRD TDD) |
| TDD rigoroso | RED-GREEN-REFACTOR |

**Nota:** Modulos complexos (ex: `sync_queue.rs`, `backup.rs`) podem ser divididos em sub-modulos se necessario para respeitar o limite de 200 linhas por arquivo.

---

## Stack de Crates Rust (Preliminar)

| Crate | Finalidade |
|-------|-----------|
| `rmcp` | SDK oficial MCP (stdio server) |
| `tantivy` | Full-text search BM25 |
| `pulldown-cmark` | Parser Markdown (seguro contra ReDoS) |
| `bincode` | Serializacao de snapshots |
| `notify` | FS watch (registro/desregistro de repos) |
| `git2` | Interacao Git nativa (diffs, commits, refs) |
| `reqwest` | HTTP client para Ollama API (a partir de M5) |
| `tokio` | Async runtime (para I/O e FS events) |
| `tracing` + `tracing-subscriber` | Logs estruturados JSON |
| `serde` + `serde_yaml` | Parse de frontmatter YAML |
| `sha2` | Hash SHA-256 para dedup de chunks |
| `wide` / `std::simd` | SIMD cosine similarity (a partir de M5) |
| `serde_json` | Serializacao JSON (logs, health check, embedding_debt) |
| `tokio-process` (ou `std::process`) | Invocacao de `git-lfs` como processo externo (T6.5) |
| `dotenvy` | Carregamento de `.env` para config e credenciais |

---

## Principio Arquitetural: Posicionamento CAP (F2.8)

O PKS se posiciona como **AP (Availability + Partition Tolerance)** no Teorema CAP:

- **Partition Tolerance (FORTE):** Repo remoto offline nao afeta repos locais. Isolamento por particao.
- **Availability (FORTE):** Double-Buffered Pipeline + Single-Thread NYC garante disponibilidade 100%. Queries sempre retornam em sub-ms.
- **Consistency (EVENTUAL OTIMISTA):** O indice nao e fonte da verdade (Git/FS e). Pequenas defasagens aceitas em nome da disponibilidade. Reparos assincrono via pipeline.

Todas as tasks devem seguir este principio: **nunca bloquear queries em nome de consistencia**.

**Tracker Sync Queue (T7.3):** Segue o mesmo principio AP. Queries sobre conteudo importado retornam dados potencialmente stale (ultima versao sincronizada). Queue aceita perda parcial com retry — operacoes falhadas voltam ao final da fila, nunca bloqueiam queries.

---

## Notas Complementares

**Estrutura do monolito:** Modulos adicionais criados nas fases estendem a estrutura base do PRD: Fase A adiciona `fifo_pipeline.rs` e `debounce.rs`; Fase B adiciona modulos Git; Fase C adiciona `fifo_embedder.rs` e `embedding_provider.rs` (M5), `observability.rs`, `git_lfs_sync.rs`, `auth.rs` (M6), `tracker/`, `storage_policy.rs` (M7). O PRD sera atualizado ao final de cada fase.

**Tech debt (Apendice B):** Protocolo de sync, versionamento de snapshots, e autenticacao entre nos estao documentados como tech debt para o escopo do Apendice B do PRD.

**Extensibilidade (Apendice C):** `search/retriever.rs` deve usar trait `SearchBackend` para permitir troca futura de flat index por HNSW. Nao implementar HNSW agora, mas preparar a abstracao. Em M5, `embedding_provider.rs` implementa trait `EmbeddingProvider` (Apendice C secao 4) para abstrair provider de embeddings (Ollama como default).

---

## Fontes de Pesquisa

- [knowledge-rag (Hybrid BM25+Semantic MCP)](https://github.com/lyonzin/knowledge-rag)
- [rust-local-rag (Rust + Ollama MCP)](https://github.com/ksaritek/rust-local-rag)
- [Rust MCP SDK oficial](https://github.com/modelcontextprotocol/rust-sdk)
- [obsidian-mcp-server](https://github.com/cyanheads/obsidian-mcp-server)
- [obsidian-mcp-tools (semantic search)](https://github.com/jacksteamdev/obsidian-mcp-tools)
- [rust-mcp-filesystem](https://github.com/rust-mcp-stack/rust-mcp-filesystem)
- [MCP Reference Servers](https://github.com/modelcontextprotocol/servers)
- [Best MCP Servers for Knowledge Bases 2026](https://desktopcommander.app/blog/best-mcp-servers-for-knowledge-bases-in-2026)
- [Awesome MCP Servers](https://github.com/TensorBlock/awesome-mcp-servers/blob/main/docs/knowledge-management--memory.md)
- [PulseMCP Directory (8600+ servers)](https://www.pulsemcp.com/servers?q=rag)
