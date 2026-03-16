# Fase C — Operação, Embeddings & Trackers (Milestones M5, M6, M7)

> Objetivo: Adicionar busca semantica via embeddings vetoriais (Ollama), estabilizar a operacao com observabilidade e backup LFS, e finalmente conectar o PKS a trackers externos (Notion, Jira).
> Pre-requisito: Fase A (M1-M3) completa. Fase B (M4) parcialmente completa.
> Nota: M5 e M6 podem iniciar apos Fase A completa; M7 (Trackers) requer a estabilidade de M6 e Fase B parcial.

---

## Milestone M5: Embeddings Vetoriais (pos-MVP)

### Objetivo
Adicionar busca semantica ao PKS via embeddings vetoriais (Ollama), incluindo pipeline hibrido (BM25 + Cosine SIMD), dedup inteligente, gestao automatica de memoria (LRU/Hibernacao), e degradacao gradual do Ollama. Este milestone transforma o PKS de BM25-only para busca hibrida completa.

> **Nota:** Movido da Fase A (M2) para Fase C conforme decisao STEERING 3.5 — MVP opera BM25-only.

### Tasks

#### T5.1e — Integracao Ollama (Embeddings)
- **Goal**: Chamar Ollama API para gerar vetores de cada chunk; implementar trait `EmbeddingProvider` para abstrir provider
- **Files**: `[CREATE] pks/src/fifo_embedder.rs`, `[CREATE] pks/src/embedding_provider.rs`
- **Tests**: Teste with Ollama local (skip se offline)
- **Acceptance**:
  - [x] Trait `EmbeddingProvider` com `fn embed(text: &str) -> Vec<f32>` (conforme PRD Apendice C secao 4)
  - [x] Implementacao `OllamaProvider` como default
  - [x] Config `PKS_EMBEDDING_PROVIDER={ollama|candle}` (extensibilidade futura)
  - [x] Gera embedding 768-D para cada chunk via modelo configuravel (`OLLAMA_EMBED_MODEL`, default `nomic-embed-text`)
  - [x] URL do Ollama configuravel via `OLLAMA_BASE_URL` (default `http://localhost:11434`)
  - [x] Throttle configuravel (`PKS_THROTTLE_MS=200`)
  - [x] Re-vetorizacao pos-commit: 2-10s para um commit medio (~10 chunks) conforme PRD
  - [x] Sub-fila `embedding_backlog` acumula chunks aguardando vetorizacao
  - [x] Overflow do backlog serializado em `embedding_debt.jsonl` (divida vetorial persistida em disco)
  - [x] Auto-healing: re-ingere divida do `embedding_debt.jsonl` quando Ollama retorna
  - [x] **Degradacao Gradual — 5 estados** (conforme PRD F2.5):
    - Nao instalado: Daemon inicia normalmente, log WARN a cada 5 min, BM25-only
    - Modelo ausente: Tenta `ollama pull nomic-embed-text` 1x no start, BM25-only se falhar
    - Temporariamente offline: `embedding_debt` na RAM, BM25 novos + hibrido historicos
    - Offline prolongado (dias): Pausa se `debt.jsonl` > 50MB, log ERROR, BM25 completo + vetorial parcial
    - Retorno apos offline: Re-ingere com throttle, prioriza recentes, convergencia gradual

#### T5.2e — Busca Hibrida (BM25 + Cosine SIMD)
- **Goal**: Combinar scores BM25 e similaridade vetorial com Reciprocal Rank Fusion
- **Files**: `[MODIFY] pks/src/search/retriever.rs`
- **Tests**: `[MODIFY] pks/tests/test_search.rs`
- **Acceptance**:
  - [x] Cosine similarity via SIMD. Default: crate `wide` (stable Rust). `std::simd` disponivel como feature flag `nightly-simd`
  - [x] Reciprocal Rank Fusion entre BM25 e vetorial
  - [x] Filtro por `projects_filter` mantido
  - [x] Golden dataset: 90%+ top-3 accuracy
  - [x] Degradacao graciosa: se vetores ausentes, retorna BM25-only
  - [x] Latencia de busca hibrida: `search_knowledge_vault` retorna resultados hibridos em < 10ms (conforme PRD M5)

#### T5.3e — Dedup por Hash de Paragrafo (Economia de Ollama)
- **Goal**: Revetorizar apenas chunks cujos paragrafos constituintes tiveram SHA-256 alterado
- **Files**: `[MODIFY] pks/src/indexer/pipeline.rs`
- **Tests**: Teste: editar 1 paragrafo em arquivo de 20+ chunks, verificar que apenas chunks dirty sao revetorizados
- **Acceptance**:
  - [x] Hash SHA-256 por paragrafo fragmentado, nao por chunk inteiro
  - [x] Chunk marcado como "dirty" se qualquer paragrafo constituinte mudou de hash
  - [x] Apenas chunks dirty vao para Ollama; demais herdam vetores estaticos
  - [x] Verificavel via metrica `pks_ollama_queue_depth`

#### T5.4e — Gestao de Memoria Automatica: LRU + Hibernacao (F2.10 evolucao)
- **Goal**: Implementar ciclo HOT -> COLD -> HIBERNADO com watermark de vetores, evoluindo a gestao manual do MVP
- **Files**: `[MODIFY] pks/src/memory_manager.rs`
- **Tests**: Teste: simular N repos, verificar hibernacao apos X dias sem query
- **Acceptance**:
  - [x] `PKS_MAX_VECTORS=500_000` — hard limit global
  - [x] `PKS_HIBERNATE_DAYS=7` — repos inativos hibernam
  - [x] Hibernacao: vetores descarregados, BM25 mantido (queries retornam BM25-only ate wake)
  - [x] Wake: vetores recarregados do snapshot em ~100ms
  - [x] LRU: repo menos usado ejetado primeiro ao bater no teto
  - [x] Evolui naturalmente da gestao manual (MVP) para gestao automatica
  - [x] Nota: Com vetores densos (768-D floats, ~3KB por chunk), gestao automatica se justifica

### Criterios de Sucesso M5
- [x] Busca hibrida retorna resultados superiores a BM25-only no golden dataset
- [x] Golden dataset: cada query de referencia retorna score minimo documentado
- [x] Ollama offline nao crash o daemon — BM25 continua operando
- [x] 5 estados de degradacao Ollama cobertos por testes
- [x] LRU funcional: repo menos usado ejetado ao atingir watermark
- [x] trait `EmbeddingProvider` permite trocar backend sem impacto na pipeline

---

## Milestone M6: Maturação e Operação de Voo

### Objetivo
Adicionar observabilidade completa e mecanismos de disaster recovery para operacao sustentavel do Daemon.

### Tasks

#### T6.1 — Logs Estruturados (F2.13)
- **Goal**: Implementar logging JSON via `tracing` com rotacao automatica
- **Files**: `[CREATE] pks/src/observability.rs`
- **Tests**: Teste: verificar formato JSON dos logs, rotacao por tamanho
- **Acceptance**:
  - [x] Formato JSON via `tracing` + `tracing-subscriber`
  - [x] Niveis: ERROR, WARN, INFO, DEBUG
  - [x] Rotacao automatica em `~/.pks/logs/` (`PKS_LOG_MAX_SIZE=50MB`)
  - [x] Eventos-chave logados: commit detectado, chunks processados, queries respondidas, erros Ollama, hibernacao/wake
  - [x] Cada log entry inclui campo `repo_id` quando aplicavel para facilitar debugging multi-repo.

#### T6.2 — Health Check Endpoint (F2.13)
- **Goal**: Endpoint `/health` com status do daemon e metricas
- **Files**: `[MODIFY] pks/src/mcp_server.rs`, `[MODIFY] pks/src/observability.rs`
- **Tests**: Teste: GET /health retorna JSON com campos esperados
- **Acceptance**:
  - [x] `GET http://localhost:{PKS_PORT}/health` (default 3030) retorna:
    - Status do daemon (running/degraded)
    - Repos registrados (warm/hibernated)
    - Profundidade das filas (fila1, fila2)
    - Uptime
  - [x] Metricas expostas conforme tabela do PRD:
    - `pks_fila1_depth`, `pks_fila2_depth`
    - `pks_query_latency_us` (p50, p95, p99)
    - `pks_repos_warm`, `pks_repos_hibernated`
    - `pks_ram_usage_bytes`
    - `pks_ollama_queue_depth`
    - `pks_last_commit_indexed` (por repo)
    - `pks_embedding_debt_entries` — numero de chunks pendentes em `embedding_debt.jsonl`
    - `pks_tracker_sync_queue_depth` — operacoes pendentes na Tracker Sync Queue

#### T6.3 — CLI de Diagnostico (F2.13)
- **Goal**: Comandos `pks status` e `pks validate` para operacao do dia-a-dia
- **Files**: `[MODIFY] pks/src/cli.rs`
- **Tests**: Testes de integracao: rodar `pks status`, verificar output
- **Acceptance**:
  - [x] `pks status`: resumo de repos, estado das filas, saude do Ollama
  - [x] `pks validate`: compara hashes SHA-256 dos `.md` com o indice; verifica ausencia de tombstones residuais no indice
  - [x] `pks validate` tambem verifica consistencia do Vector Clock contra HEADs reais dos repos Git registrados.
  - [x] Output formatado e legivel no terminal
  - [x] Exit codes corretos (0 = ok, 1 = issues encontradas)

#### T6.4 — Daemon como Servico de SO (F2.2)
- **Goal**: Configurar PKS como servico `launchd` (macOS) e `systemd` (Linux)
- **Files**: `[CREATE] pks/deploy/com.pks.daemon.plist` (launchd), `[CREATE] pks/deploy/pks.service` (systemd)
- **Tests**: Teste manual: instalar servico, verificar auto-start
- **Acceptance**:
  - [x] `launchd` plist para macOS with auto-restart; `WorkingDirectory` apontando para diretorio com `.env` para garantir `dotenvy` discovery
  - [x] `systemd` unit file para Linux com `WorkingDirectory=` e `EnvironmentFile=` apontando para `.env`
  - [x] Ambos configs garantem que `PKS_PORT`, `PKS_VAULTS_DIR`, `OLLAMA_BASE_URL`, `OLLAMA_EMBED_MODEL` e credenciais de trackers estejam disponiveis no ambiente do daemon
  - [x] Logs direcionados para `~/.pks/logs/`
  - [x] Graceful shutdown ao receber SIGTERM: daemon drena Fila 2 (mutacoes pendentes aplicadas), serializa Fila 1 residual em disco (`~/.pks/fila1_drain.jsonl`), e salva snapshots de todos os repos WARM. Tracker Sync Queue serializa operacoes pendentes em disco (`~/.pks/sync_queue_drain.jsonl`). Ao reiniciar, filas drenadas sao re-ingeridas.
  - [x] macOS: `KeepAlive=true` (reinicia em crash) + `RunAtLoad=true` (inicia no boot). Linux: `Restart=always` + `WantedBy=multi-user.target`.
  - [x] Documentacao de instalacao (inclui instrucoes de configuracao de env vars para service mode)

#### T6.5 — Contingencia e Disaster Recovery (F2.14)
- **Goal**: Sincronizacao de snapshots vetoriais via Git LFS em repositorio satélite dedicado (Isolamento 1:1) e procedimento de recuperacao documentado
- **Files**: `[CREATE] pks/src/git_lfs_sync.rs`, `[CREATE] docs/DISASTER_RECOVERY.md`
- **Tests**: Teste: sincronizar snapshot para repo Git LFS local, restaurar em "maquina nova" via `git clone`
- **Acceptance**:
  - [x] Espelhamento periodico de `snapshots/<repo_id>.bin`: Daemon detecta se `PKS_VECTOR_REMOTE_URL` esta configurada (URL exclusiva por projeto)
  - [x] Se configurada, o daemon inicializa um repositorio Git satelite em `~/.pks/snapshots/`, aplica `.gitattributes` com `*.bin filter=lfs diff=lfs merge=lfs -text`
  - [x] Daemon executa `git lfs track "*.bin"`, realiza lock/unlock via LFS (se aplicavel), comita com `--amend` ou Orphan Branch para evitar acumulo de historico, e faz push focado no sub-repo LFS
  - [x] Interface de sincronizacao abstrata: `trait SnapshotStore` com implementacoes `LocalStore` e `GitLfsStore`
  - [x] Procedimento documentado: `git clone` do repo satelite LFS = snapshots restaurados **sem re-vetorizacao** (conforme PRD Fase 2 acceptance)
  - [x] Sem `PKS_VECTOR_REMOTE_URL`: `git clone` do repo principal + reindex completo funciona (BM25 imediato, vetores convergem via Ollama)
  - [x] Teste de restore: clonar repo satelite LFS + carregar snapshots = indice funcional sem chamar Ollama
  - [x] Sincronizacao incremental: apenas snapshots modificados desde ultimo push sao enviados (Git LFS rastreia por conteudo automaticamente)
  - [x] Compressao zstd opcional antes de commit. Configuravel via `PKS_BACKUP_COMPRESS=true`
  - [x] Reidratacao sob demanda em segundos (conforme PRD F2.14): `git lfs pull` de snapshot individual < 5s por repo
- [x] **Implementar "Pull on Startup"**: Ao iniciar (ou registrar novo repo), o daemon verifica se existe snapshot local; se ausente e `PKS_VECTOR_REMOTE_URL` configurada, realiza `git pull` automático do repositório LFS para restaurar o índice sem re-vetorização.
  - [x] Tempo de recuperacao documentado por cenario (com e sem repo satelite), com benchmarks medidos
  - [x] Benchmarks documentados por cenario: (a) restore local (snapshot em disco): < 5s por repo; (b) restore via Git LFS (latencia de rede): benchmark medido e documentado em `docs/DISASTER_RECOVERY.md` com condicoes de teste
  - [x] Cenarios de falha: `git-lfs` nao instalado ou limite de cota LFS estourado na origem = log ERROR + rollback gracioso para vetorizacao local-only. Degradacao graciosa: falha de sincronizacao nunca impacta operacao do daemon
  - [x] Estrategia de amortizacao de storage: uso de Orphan Branches ou Squash no repo satelite para manter apenas o snapshot mais recente vivo, evitando acumulo de bytes cobrados pelo provedor LFS
  - [x] Compativel com dependencia do Apendice B (PKS Team Node) que assume snapshots distribuidos via Git LFS

#### T6.6 — Seguranca MCP (F2.12)
- **Goal**: Garantir que o servidor MCP opera de forma segura em modo single-player
- **Files**: `[MODIFY] pks/src/mcp_server.rs`, `[CREATE] pks/src/auth.rs`
- **Tests**: Teste: conexao de IP externo recusada; credenciais nao no Git
- **Acceptance**:
  - [x] Servidor escuta apenas em `127.0.0.1:{PKS_PORT}` (default 3030)
  - [x] Conexoes externas recusadas por padrao
  - [x] Credenciais de trackers via env vars ou keychain
  - [x] Parser Markdown: timeout e limite de aninhamento contra ReDoS
  - [x] Auditoria de artefatos em disco: `embedding_debt.jsonl` validado antes de re-ingestao (verificacao de integridade do JSON, rejeicao de payloads malformados). Snapshots `.bin` validados via Magic Version Header antes de load.
  - [x] Tech Debt documentado: tokens, TLS, CORS, multi-tenant adiados (conforme PRD F2.12)

### Criterios de Sucesso M6
- [x] `pks status` mostra panorama completo do daemon
- [x] `pks validate` detecta drift entre FS e indice
- [x] Health check retorna metricas em formato JSON
- [x] Daemon roda como servico de SO com auto-restart
- [x] Sincronizacao/restore funcional via Git LFS em repo satelite; `git clone` do repo satelite = snapshots restaurados sem re-vetorizacao
- [x] Sem repo satelite tambem funcional: `git clone` do repo principal + reindex completo (BM25 imediato, vetores via Ollama)
- [x] Zero portas expostas externamente

---

## Milestone M7: Trackers Contextuais (Expansão)

### Objetivo
Implementar import/export bidirecional entre trackers externos e o vault `prometheus/`, com fila de sincronizacao nao-bloqueante.

### Tasks

#### T7.1 — Import de Tracker Externo (F1.3)
- **Goal**: Puxar dados de tracker (Notion/Jira) e gerar `.md` com frontmatter YAML no `prometheus/`
- **Files**: `[CREATE] pks/src/tracker/import.rs`, `[CREATE] pks/src/tracker/mod.rs`
- **Tests**: Teste de integracao com tracker real (Notion via MCP); skip se offline (sem mocks — conforme criterio global "Zero mocks")
- **Invocacao**: Expor como MCP tool `pks_import_tracker(tracker_id, tracker_type?)` + CLI `pks import <tracker_id>` para uso direto
- **Acceptance**:
  - [x] Input: ID do ticket (ex: `PAY-4421`)
  - [x] Output: arquivo `.md` em `prometheus/02-features/` com frontmatter:
    ```yaml
    tracker_id: PAY-4421
    tracker: jira
    status: in_progress
    tags: [checkout, timeout, backend]
    synced_at: 2026-03-06T14:00:00Z
    source_commit_sha: <SHA do HEAD de main no momento do import, quando aplicavel>
    ```
  - [x] Commit Linking: SHA do commit atual de `main` injetado no frontmatter YAML quando aplicavel (conforme F1.2)
  - [x] Nota: `source_commit_sha` e injetado quando o import e feito a partir de um repo Git com branch `main` ativa. Quando import e feito fora de contexto Git (ex: via CLI standalone), campo omitido.
  - [x] Commit automatico em `pks-knowledge`
  - [x] Pelo menos 1 tracker funcional end-to-end (Notion via MCP existente)
  - [x] Diretorio destino inferido pelo tipo de conteudo ou configuravel
  - [x] Logica de mapeamento de diretorio destino: regras configuraveis via arquivo `prometheus/.pks-routing.yaml` (ex: `jira_bug -> 02-features/{component}/`, `github_issue -> 05-decisions/`). Default: `02-features/` se tipo nao mapeado. Exemplos do PRD cobertos (bug Jira, issue GitHub, post Slack).
  - [x] Invocavel via MCP tool (pelo Antigravity) e via CLI (pelo usuario)
- **MCP Reference**: Usar `@notionhq/notion-mcp-server` ja configurado no projeto para Notion; estudar `server-git` para commits

#### T7.2 — Export para Tracker Externo (F1.4)
- **Goal**: Publicar `.md` local de volta no tracker, atualizando metadados
- **Files**: `[CREATE] pks/src/tracker/export.rs`
- **Tests**: Teste de integracao com tracker real (Notion)
- **Invocacao**: Expor como MCP tool `pks_export_tracker(file_path, tracker_type?)` + CLI `pks export <file_path>`
- **Acceptance**:
  - [x] Input: caminho do arquivo `.md`
  - [x] Le frontmatter para determinar tracker destino; se frontmatter ausente, usa tracker default configurado (conforme PRD F1.4)
  - [x] Publica conteudo via MCP do tracker
  - [x] Atualiza frontmatter com ID gerado e `synced_at`
  - [x] Commit da atualizacao de frontmatter em `pks-knowledge` (nao polui `main`)
  - [x] Suporte a criacao de pagina nova E atualizacao de pagina existente (determinado pelo `tracker_id` no frontmatter). Cenarios de referencia do PRD: ADR -> Notion page, spec -> tracker ticket, rascunho -> GitHub Issue.
  - [x] Funciona para pelo menos 1 tracker (Notion)
  - [x] Collision Detection (OCC): antes de publicar, PKS verifica `updated_at` no tracker destino. Se versao remota for mais nova que `synced_at` local, export falha com alerta ao usuario (prevencao de Mid-Air Collision conforme PRD F1.4)
  - [x] Invocavel via MCP tool (pelo Antigravity) e via CLI (pelo usuario)

#### T7.3 — Tracker Sync Queue (F1.6)
- **Goal**: Fila FIFO de sincronizacao com consumidor dedicado, retry com backoff exponencial
- **Files**: `[CREATE] pks/src/tracker/sync_queue.rs`
- **Tests**: `[CREATE] pks/tests/test_tracker_sync.rs`
- **Invocacao**: Expor como MCP tool `pks_enqueue_sync(operations: list)` + CLI `pks sync-queue <add|status|flush>`
- **Acceptance**:
  - [x] Fila FIFO independente do Daemon principal
  - [x] Nao bloqueia: Antigravity enfileira e continua
  - [x] Lote nativo: "importe 30 tickets" = 30 entradas na fila
  - [x] Retry com backoff exponencial em falha de rede
  - [x] Consumidor respeita rate limits do tracker: le headers `Retry-After` / `X-RateLimit-*` e pausa automaticamente (conforme PRD F1.6 "respeitando rate limits")
  - [x] Operacao falhada nao perde — volta ao final da fila
  - [x] Cenarios de falha de tracker API: timeout de rede (retry com backoff), auth expirado/token revogado (log ERROR + operacao movida para dead-letter; nao bloqueia fila), rate limit excedido (pausa automatica via headers)
  - [x] Separada da Fila 1 do Daemon (alimenta indiretamente via commits Git)
  - [x] Nota de design: Tracker Sync Queue implementada como thread dedicada dentro do binario `pks` (nao processo separado do OS). O PRD usa 'processo irmao' no sentido de independencia logica, nao de PID separado. A thread tem seu proprio loop de eventos, desacoplada do pipeline principal. Justificativa: simplifica deploy (binario unico) e compartilha `.env`/config.
  - [x] Credenciais via env vars ou keychain do SO
  - [x] Invocavel via MCP tool (pelo Antigravity) e via CLI (pelo usuario)

#### T7.4 — Sanitizacao de Conteudo Importado (F2.12)
- **Goal**: Limpar HTML inline, scripts e links maliciosos do conteudo importado
- **Files**: `[CREATE] pks/src/tracker/sanitizer.rs`
- **Tests**: Teste with payloads maliciosos (XSS, script tags, etc.)
- **Acceptance**:
  - [x] Remove HTML inline e script tags
  - [x] Remove links potencialmente maliciosos
  - [x] Output e Markdown puro
  - [x] Nao altera conteudo legitimo (links normais, formatacao MD)
  - [x] Limite maximo de conteudo importado: `PKS_IMPORT_MAX_SIZE=1MB` (default). Conteudo acima do limite e truncado com aviso no log.

#### T7.5 — Armazenamento Seletivo (F1.5)
- **Goal**: Definir e implementar politica system-wide de o que vai/nao vai ao `prometheus/` — abrange imports de tracker, resumos AI (`90-ai-memory`), notas manuais e qualquer conteudo automatizado
- **Files**: `[CREATE] pks/src/storage_policy.rs`, `[CREATE] docs/STORAGE_POLICY.md`
- **Tests**: Testes unitarios de filtragem para cada tipo de conteudo
- **Acceptance**:
  - [x] Regras configuraveis por projeto (ex: ignorar dumps brutos)
  - [x] Default: importar contexto e raciocinio, nao dados brutos (conforme PRD F1.5)
  - [x] Filtros por tipo de conteudo, tamanho, tags
  - [x] Politica cobre todos os tipos de conteudo: imports de tracker, resumos AI de sessao, ADRs, notas manuais, runbooks
  - [x] Exemplos documentados por tipo de repositorio (engenharia, academico, pessoal) conforme tabela do PRD F1.5
  - [x] Minimo 5 exemplos documentados (engenharia, academico, pessoal, open-source, pesquisa) conforme tabela do PRD F1.5.
  - [x] Documentacao clara do principio: "prometheus/ guarda contexto e raciocinio, nao a informacao em si"

### Criterios de Sucesso M7
- [x] Import Notion end-to-end: ID do ticket -> `.md` commitado com frontmatter correto
- [x] Export Notion end-to-end: `.md` local -> pagina publicada no Notion
- [x] Sync Queue processa 30 operacoes sem bloquear o Antigravity
- [x] Retry funciona: simular falha de rede, verificar re-tentativa
- [x] Conteudo importado sanitizado: zero HTML/scripts no `.md` final
- [x] Credenciais nunca versionadas no Git
- [x] Politica de armazenamento seletivo (T7.5) documentada e funcional: cobre imports, resumos AI, notas manuais; `docs/STORAGE_POLICY.md` publicado

---

## Dependencias Externas (Fase C)

| Dependencia | Instalacao | Obrigatoria? |
|-------------|-----------|--------------|
| Ollama | `brew install ollama` + `ollama pull nomic-embed-text` | Sim (M5) |
| Notion MCP Server | Ja configurado (`@notionhq/notion-mcp-server`) | Sim (T7.1, T7.2) |
| NOTION_TOKEN | Env var | Sim |
| git-lfs | `brew install git-lfs && git lfs install` | Recomendado (T6.5 — M6) |

---

## Riscos e Mitigacoes

| Risco | Impacto | Mitigacao |
|-------|---------|-----------|
| Ollama indisponivel ou modelo com regressao | Vetorizacao bloqueada | BM25-only fallback; divida vetorial em disco; 5 estados de degradacao |
| SIMD nao disponivel em todas as CPUs | Cosine similarity lento | Fallback para implementacao scalar; feature flag para SIMD |
| API do Notion com rate limits agressivos | Import/Export lento | Sync Queue com backoff exponencial + respeito a headers de rate limit |
| Jira/Linear nao tem MCP server pronto | Bloqueio de multi-tracker | Focar em Notion primeiro; Jira/Linear como extensao futura |
| Snapshots grandes (repos com muitos vetores) | Push LFS lento | Compressao zstd antes do commit; Orphan Branches para evitar acumulo de historico LFS |
| Cota LFS estourada no provedor (GitHub/GitLab) | Sincronizacao bloqueada | Degradacao graciosa para vetorizacao local-only; estrategia de squash/orphan para minimizar uso de cota |
| launchd/systemd config fragil | Daemon nao reinicia | Testes de smoke em CI; documentacao detalhada |

---

## Pos-Fase C: O que vem depois?

Apos a conclusao da Fase C (M5-M7), o PKS tera:
- Motor de RAG prevalente em Rust com busca hibrida (BM25 + embeddings vetoriais)
- Integracao Git nativa com deteccao automatica de mudancas
- Import/Export de pelo menos 1 tracker (Notion)
- Observabilidade e operacao sustentavel
- Degradacao gracioso do Ollama com 5 estados documentados

**Proximos passos** (conforme Apendices do PRD):

| Apendice | Tema | Dependencia |
|----------|------|-------------|
| **A** | Shadow Repositories (Slack, WhatsApp, Email, codigo-fonte de projetos de software como Shadow Git Repos) | Fases 1-2 estaveis |
| **B** | PKS Team Node (No Semente Colaborativo em cloud/on-premise) | F2.14 (Disaster Recovery com Git LFS) |
| **C** | Escala Extrema e Eficiencia Hibrida (HNSW O(log N), Battery Awareness, Cloud Offload de vetorizacao) | Pressoes de crescimento exponencial pos-Fase 2 |

Todos serao detalhados em PRDs proprios conforme especificado no documento original.
