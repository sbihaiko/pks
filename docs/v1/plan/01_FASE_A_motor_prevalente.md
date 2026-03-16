# Fase A — Motor Prevalente de RAG (Milestones M1, M2, M3)

> Objetivo: Construir o core do PKS Daemon em Rust — desde o servidor MCP basico ate a persistencia completa com gestao manual de memoria.

---

## Milestone M1: Engine Rustica

### Objetivo
Servidor MCP stdio funcional que parseia Markdown, indexa via BM25 (Tantivy), e responde queries de um diretorio raiz de vaults.

### Tasks

#### T1.1 — Scaffold do Projeto Rust ✅ CONCLUIDA
- **Goal**: Criar o repositorio `pks/` com Cargo.toml, estrutura de modulos conforme PRD, CI basico
- **Files**: `[CREATE] pks/Cargo.toml`, `[CREATE] pks/src/main.rs`, `[CREATE] pks/src/lib.rs`, `[CREATE] pks/src/state.rs`, `[CREATE] pks/tests/golden_dataset/`
- **Acceptance**:
  - [x] `cargo build` compila sem erros
  - [x] `cargo test` roda (mesmo sem testes reais ainda)
  - [x] Estrutura de modulos conforme secao "Estrutura do Monolito" do PRD
  - [x] `state.rs` criado com struct `PrevalentState` basica (placeholder para indices BM25 e vetores)
  - [x] `state.rs` implementa struct `PrevalentState` conforme "Modelo de Dados High-Level" do PRD (secao 4.1): `repos: HashMap<RepoId, RepoIndex>`, `vector_clock`, `embedding_debt`, `global_stats`. Campos vetoriais (`embedding: Option<Vec<f32>>`) presentes como `None` no MVP (BM25-only)
  - [x] Diretorio `tests/golden_dataset/` criado com 10 notas `.md` iniciais de referencia
- **MCP Reference**: Estudar `rmcp` SDK examples para setup inicial

#### T1.2 — Parser Markdown + Chunking Semantico (F2.4) ✅ CONCLUIDA
- **Goal**: Implementar chunking por heading com fallback sliding window (400t/80t overlap)
- **Files**: `[CREATE] pks/src/indexer/chunker.rs`, `[CREATE] pks/src/indexer/pipeline.rs`
- **Tests**: `[CREATE] pks/tests/test_chunking.rs`
- **Acceptance**:
  - [x] Split por headings (`##`, `###`) funciona
  - [x] Sliding window para secoes > `PKS_CHUNK_MAX_TOKENS` (default 400)
  - [x] Agrupamento de secoes < `PKS_CHUNK_MIN_TOKENS` (default 100)
  - [x] Overlap configuravel via `PKS_CHUNK_OVERLAP_TOKENS` (default 80)
  - [x] Os 3 parametros lidos de env vars (conforme PRD F2.4); valores hardcoded apenas como defaults. **Nota**: env vars para chunk params sao extensao do PRD (o PRD define valores fixos; aqui parametrizamos para flexibilidade)
  - [x] Hash SHA-256 por fragmento de paragrafo para dedup (granularidade a nivel de paragrafo conforme PRD F2.4, nao apenas a nivel de chunk)
  - [x] Geracao de evento Tombstone quando arquivo `.md` e deletado do repositorio (evento propagado para pipeline T2.2)
  - [x] Metadados preservados: `(repo_id, file_path, heading_hierarchy, chunk_index, chunk_hash)`
  - [x] Parser usa `pulldown-cmark` (seguro contra ReDoS)
  - [x] Timeout maximo por varredura configuravel via `PKS_PARSE_TIMEOUT_MS` (default 5000)
  - [x] Limite de aninhamento sintatico para prevenir ReDoS configuravel via `PKS_PARSE_MAX_NESTING` (default 100) (conforme F2.12)
- **MCP Reference**: Estudar chunking do `knowledge-rag` e `mcp-local-rag`

#### T1.3 — Indice BM25 com Tantivy ✅ CONCLUIDA
- **Goal**: Indexar chunks em Tantivy, suportar queries full-text com scoring BM25
- **Files**: `[CREATE] pks/src/search/retriever.rs`
- **Tests**: `[CREATE] pks/tests/test_search.rs`
- **Acceptance**:
  - [x] Adicionar/remover documentos do indice
  - [x] Query retorna top-k resultados com score
  - [x] Filtro por `repo_id` funciona
  - [x] Latencia < 1ms para 1000 chunks
  - [x] Garbage Collection: Tombstones removem chunks de arquivos deletados
  - [x] Implementar `trait SearchBackend` em `retriever.rs` para abstrir o backend de busca (extensibilidade futura para HNSW conforme Apendice C). Implementacao inicial: `TantivyBackend`

#### T1.4 — Filesystem-as-Config (F2.11) ✅ CONCLUIDA
- **Goal**: Monitorar diretorio raiz (`~/pks-vaults/`) para auto-registro de repos Git
- **Files**: `[CREATE] pks/src/repo_watcher.rs`
- **Tests**: Teste de integracao: criar/deletar diretorio e verificar registro
- **Acceptance**:
  - [x] Diretorio raiz lido de `PKS_VAULTS_DIR` env var (default `~/pks-vaults` se ausente)
  - [x] `git clone` em `{PKS_VAULTS_DIR}/` = auto-registro (COLD -> WARM); suporta tambem referencia local via `file://` — isto significa que alem do diretorio raiz monitorado, repos podem ser referenciados manualmente via path `file:///path/to/repo` (uso avancado conforme PRD F2.7)
  - [x] `rm -rf` = auto-purge da RAM **e** do snapshot em disco (`snapshots/<repo_id>.bin` deletado) conforme PRD F2.11
  - [x] Usa crate `notify` para FS watch
  - [x] Detecta `.git/` como marcador de repositorio valido

#### T1.5 — Servidor MCP stdio Basico (F2.2) ✅ CONCLUIDA
- **Goal**: Expor `search_knowledge_vault(query, top_k, projects_filter)` via MCP stdio
- **Files**: `[CREATE] pks/src/mcp_server.rs`
- **Tests**: `[CREATE] pks/tests/test_mcp_e2e.rs`
- **Acceptance**:
  - [x] Servidor opera via transporte stdio (`pks --stdio`) — sem porta de rede exposta
  - [x] Tool `search_knowledge_vault` responde com resultados BM25 em < 1ms consumindo zero disco (RAM-only)
  - [x] Contrato de resposta da tool `search_knowledge_vault` documentado (schema JSON com campos: `results[]`, cada um com `file_path`, `heading_hierarchy`, `chunk_text`, `score`, `repo_id`)
  - [x] Filtro por projetos funciona
  - [x] Usa `rmcp` SDK
  - [x] Diretorio raiz de vaults lido de `PKS_VAULTS_DIR` env var (injetada pelo `.mcp.json`)
  - [x] Tool MCP `list_knowledge_vaults()` exposta: retorna lista de `RepoId` registrados e WARM. Essencial para LLM "olhar ao redor" e descobrir contexto da maquina (conforme PRD F2.2)
  - **Nota**: `PKS_PORT` planejada para SSE foi removida — transporte stdio nao usa porta de rede
- **MCP Reference**: Estudar setup stdio do `rust-local-rag`

#### T1.6 — Golden Dataset Completo + Harness de Integracao ✅ CONCLUIDA
- **Goal**: Expandir golden dataset para 50 notas com queries de referencia e score floors; configurar harness `cargo test --features integration`
- **Files**: `[CREATE] pks/tests/golden_dataset/*.md` (50 notas), `[CREATE] pks/tests/golden_queries.toml` (queries + expected top-3 + score floors), `[MODIFY] pks/Cargo.toml` (feature flag `integration`)
- **Tests**: `cargo test --features integration` roda o dataset completo
- **Acceptance**:
  - [x] 50 notas `.md` de referencia cobrindo dominios variados (engenharia, academico, pessoal)
  - [x] Arquivo `golden_queries.toml` com pelo menos 20 queries de referencia, cada uma com top-3 esperado e score minimo (score floor)
  - [x] Feature flag `integration` no `Cargo.toml` separa testes unitarios de testes de integracao
  - [x] `cargo test --features integration` executa pipeline completo: `.md` -> chunk -> indice -> query -> resultado validado contra score floor
  - [x] Pronto antes do inicio de M2 (pre-requisito para validacao de pipeline BM25)

### Criterios de Sucesso M1
- [x] `cargo test` passa todos os testes
- [x] Golden dataset (10 notas iniciais): queries basicas retornam resultados corretos
- [x] Golden dataset expandido para 50 notas com score floors (T1.6) antes do inicio de M2
- [x] `cargo test --features integration` executa e passa
- [x] MCP client consegue conectar e buscar via stdio
- [x] Daemon inicia, descobre repos em `~/pks-vaults/`, indexa `.md` e responde queries
- [x] Atualizacao BM25 pos-commit: sub-100ms (atomico, single-thread) conforme PRD
- [ ] **Nota F2.2**: Daemon como servico de SO (launchd/systemd) e escopo do M7 (T7.4); na Fase A o daemon e iniciado manualmente
- [ ] **Nota F2.1**: Na Fase A, o daemon faz indexacao inicial por full-scan do diretorio raiz; deteccao de mudancas granular (post-commit hook + FS events) e implementada na Fase B (T4.4 em M4). O pipeline T2.2 recebe eventos do `repo_watcher` (T1.4) para registros/desregistros e pode receber re-scans manuais

---

## Milestone M2: Pipeline NYC-Style BM25-only

### Objetivo
Implementar pipeline double-buffered NYC-style para ingestao assincrona de commits, com processamento BM25-only (sem Ollama, sem embeddings no MVP).

### Tasks

#### T2.2 — Double-Buffered Pipeline NYC-Style (F2.3) ✅ CONCLUIDA
- **Goal**: Implementar Fila 1 (transacoes brutas) + Consumidor BG + Fila 2 (mutacoes prontas) + Thread Principal
- **Files**: `[MODIFY] pks/src/state.rs`, `[CREATE] pks/src/fifo_pipeline.rs`, `[CREATE] pks/src/debounce.rs`
- **Tests**: `[CREATE] pks/tests/test_backpressure.rs`
- **Acceptance**:
  - [x] Fila 1: limite `PKS_FILA1_MAX=1000`, tolerancia a perda
  - [x] Entrada na Fila 1 e idempotente: mesmo Commit SHA ou Tree Hash em janela de debounce = 1 entrada. Modulo `debounce.rs` criado provisoriamente na Fase A; versao completa com hooks em M4 (T4.4)
  - [x] Fila 2: limite `PKS_FILA2_MAX=500`, backpressure no consumidor
  - [x] Thread principal: queries em serie < 1ms, aplica Fila 2 em janelas de ociosidade
  - [x] Thread principal varre RAM e ejeta chunks de arquivos deletados ao processar Tombstone da Fila 2 (conforme F2.4). Tombstones compactados no snapshot em T3.1
  - [x] Thread BG faz: parse diff, chunking, atualizacao de indice BM25. Sem chamadas a LLM ou Ollama no MVP
  - [x] Sem mutexes bloqueantes na thread de queries
  - [x] Swap atomico do indice funciona sem corrompimento
  - [x] **F2.8 AP verificavel**: query respondida em < 1ms mesmo durante ingestao pesada (Fila 1 saturada); teste de integracao confirma que queries nunca bloqueiam esperando pipeline

### Criterios de Sucesso M2
- [x] Pipeline NYC-style funcional: queries nunca bloqueiam durante indexacao
- [x] Golden dataset: cada query de referencia retorna score minimo documentado (score floors por query no dataset)
- [x] MCP E2E: resposta do `search_knowledge_vault` validada contra schema JSON/Markdown esperado (conforme PRD TDD)
- [x] Backpressure: filas respeitam limites sem perda de dados criticos

---

## Milestone M3: Persistencia e Resiliencia

### Objetivo
Snapshots segmentados, Vector Clock, gestao manual de memoria, e tolerancia a falhas completa.

### Tasks

#### T3.1 — Snapshots Segmentados (D12) ✅ CONCLUIDA
- **Goal**: Serializar estado por repo em `snapshots/<repo_id>.bin` com Magic Version Header
- **Files**: `[CREATE] pks/src/snapshot.rs`
- **Tests**: Teste: salvar snapshot, recarregar, verificar igualdade; teste de version mismatch
- **Acceptance**:
  - [x] Formato bincode com Magic Version Header: 4 bytes magic (`PKS\0`), 4 bytes versao (u32 little-endian), 32 bytes schema hash (SHA-256 do layout da struct serializada). Mismatch em qualquer campo = delete + reindex
  - [x] Um arquivo `.bin` por repositorio (lazy load)
  - [x] Cadencia default: snapshot salvo a cada 100 commits processados ou a cada 5 minutos de inatividade (o que vier primeiro). Configuravel via `PKS_SNAPSHOT_INTERVAL_COMMITS` e `PKS_SNAPSHOT_INTERVAL_SECS`
  - [x] Mismatch de versao = delete + reindex do zero
  - [x] Compactacao: Tombstones expurgados ao salvar (referencia de T1.2 e T2.2 para geracao/processamento de tombstones)
  - [x] Cold start: carrega sob demanda na primeira query

#### T3.2 — Vector Clock Multi-Repo (F2.9) ✅ CONCLUIDA
- **Goal**: Rastrear `{(repo_id, branch): commit_sha}` para cada snapshot, garantir determinismo por namespace e branch
- **Files**: `[MODIFY] pks/src/state.rs`
- **Tests**: Teste: simular 3 repos com multiplas branches e commits intercalados, verificar vector clock correto por (repo, branch)
- **Acceptance**:
  - [x] Cada snapshot grava vector clock `{(repo_id, branch): commit_sha}` — rastreia HEADs de TODAS as branches monitoradas por repo, nao apenas um SHA unico por repo. Na Fase A: `main` e `feature/*`; branch `pks-knowledge` adicionada ao clock quando criada em M4 (T4.2). A estrutura de dados e branch-aware desde M3 para evitar refatoracao
  - [x] Mutacoes na Fila 2 estampadas com tupla `(Timestamp_Ingestao_Daemon, Repo_ID, Branch, Commit_SHA)` conforme F2.9
  - [x] Namespacing: commits de Repo A nao colidem com Repo B; commits de branches diferentes do mesmo repo sao rastreados independentemente
  - [x] Reidratacao tolerante no reboot
  - [x] Rebase detectado = reindex do repo afetado
  - [x] Teste: aplicar mutacoes de 3 repos (com multiplas branches cada) em ordens diferentes, verificar que estado final e identico (determinismo por namespace e branch)

#### T3.3 — Gestao de Memoria Manual (F2.10) ✅ CONCLUIDA
- **Goal**: Implementar gestao de memoria manual conforme PRD F2.10: Clone = Carregar, Delete = Descarregar. Sem LRU, sem watermarks, sem hibernacao no MVP.
- **Files**: `[CREATE] pks/src/memory_manager.rs`
- **Acceptance**:
  - [x] Clone em `~/pks-vaults/` = auto-carga (indices BM25 criados em RAM)
  - [x] `rm -rf` = auto-descarga (RAM e snapshot purgados — complementa T1.4)
  - [x] Sem LRU automatico, sem `PKS_MAX_VECTORS`, sem `PKS_HIBERNATE_DAYS`
  - [x] Com BM25-only, consumo de RAM e irrisorio (dezenas de KB por 1000 notas conforme PRD)
  - [x] Se maquina pressionada, dev remove projeto de `~/pks-vaults/` manualmente
  - [x] Nota: LRU automatico, watermarks e hibernacao (HOT/COLD) adiados para pos-MVP (Fase C, Milestone de Embeddings) quando vetores densos justificam gestao automatica

#### T3.4 — Tolerancia a Falhas e Reconstituicao (F2.5, D5) ✅ CONCLUIDA
- **Goal**: Garantir recuperacao em todos os cenarios de falha, incluindo reconstituicao a partir do historico Git (Git-as-Journal conforme D5)
- **Files**: `[MODIFY] pks/src/state.rs`, `[MODIFY] pks/src/snapshot.rs`
- **Tests**: Testes de integracao para cada cenario
- **Acceptance**:
  - [x] Reboot: load snapshot + reconciliacao com HEAD
  - [x] Snapshot corrompido: delete + reindex
  - [x] Branch checkout: invalidacao + recarga
  - [x] Rebase agressivo: Drop & Rebuild do repo
  - [x] Git-Journal Replay (D5): sem snapshot, reconstruir indice BM25 completo varrendo todos os `.md` na HEAD atual do repo — a HEAD e o resultado acumulado do Git Journal Prevalente (cada commit foi uma transacao no diario); opcionalmente, usar `git log` para reconstruir metadados temporais (timestamps de ingestao original) quando disponiveis
  - [x] Teste explicito: deletar todos os snapshots, reiniciar daemon, verificar que indice e reconstituido corretamente a partir da HEAD do Git
  - [x] Teste adicional: comparar indice reconstituido com indice original (pre-delete) — devem ser funcionalmente equivalentes (mesmos chunks, mesmos hashes, mesmos resultados de query)
  - [x] Teste: reconstruir indice SEM usar `git log` (apenas HEAD) — deve funcionar; Teste: reconstruir COM `git log` — metadados temporais devem estar corretos
  - [x] **F2.8 AP verificavel**: durante reconstituicao pos-falha, queries BM25 continuam respondendo (disponibilidade > consistencia); eventual consistency aceita — indice converge assincronamente

### Criterios de Sucesso M3
- [x] Daemon sobrevive a restart sem perda de dados
- [x] Cold start de 10 repos < 5 segundos (lazy load)
- [x] Reindex completo do vault (1000 notas) converge em < 2 minutos (BM25 puro, conforme PRD M4)
- [x] Gestao manual de memoria funciona: clone = carga, delete = descarga
- [x] Rebase nao corrompe o indice — rebuild automatico
- [x] Todos os cenarios de falha cobertos por testes de integracao

---

## Notas de Escopo e Adiamentos (Fase A)

- **Modulos de fases posteriores**: `cli.rs`, `auth.rs`, `git_journal.rs`, `observability.rs` sao criados em fases posteriores (M4/M6/M7). `debounce.rs` e criado provisoriamente em M2 (T2.2).
- **Ollama/Embeddings adiados**: Ollama, embeddings vetoriais, busca hibrida (BM25 + Cosine), e dedup por hash de paragrafo para economia de Ollama sao escopo da Fase C (novo M5 de Embeddings). Fase A opera exclusivamente com BM25.
- **LRU/Hibernacao adiados**: LRU automatico, watermarks (`PKS_MAX_VECTORS`), e hibernacao (HOT/COLD/HIBERNADO) adiados para Fase C (Milestone de Embeddings) quando vetores densos justificam gestao automatica de memoria.
- **Sanitizacao de .md locais**: Sanitizacao de conteudo `.md` local nao e escopo da Fase A; conteudo importado de trackers e sanitizado em M6 (T6.4).
- **Keychain do SO adiado**: Keychain do SO (macOS `security`, Linux `secret-tool`) adiado para fases posteriores; Fase A usa exclusivamente `.env`/`dotenvy`.
- **Health check adiado**: Health check endpoint (`/health`) adiado para M7 (T7.2).
- **CLI de diagnostico adiado**: `pks status` e `pks validate` adiados para M7 (T7.3).

---

## Riscos e Mitigacoes (Fase A)

| Risco | Impacto | Mitigacao |
|-------|---------|-----------|
| Rust toolchain instavel ou breaking changes em crates | Build falha | Fixar versoes no Cargo.toml; usar `rust-toolchain.toml` com versao especifica |
| Tantivy API breaking change entre versoes | Indice BM25 incompativel | Fixar versao no Cargo.toml; testes de integracao cobrem schema |
| `notify` crate com bugs em macOS ou Linux especificos | FS watch falha | Polling de baixa frequencia (30s) como fallback; testes em CI multi-OS |
| Golden dataset insuficiente para validar qualidade | Falsos positivos nos testes | Expandir dataset de 10 para 50 notas antes de M2; curar queries manualmente |

---

## Dependencias Externas (Fase A)

| Dependencia | Instalacao | Obrigatoria? |
|-------------|-----------|--------------|
| Rust toolchain | `rustup` | Sim |
| Diretorio `~/pks-vaults/` | `mkdir ~/pks-vaults` | Sim |
| Git repos de teste | Clone qualquer repo com `.md` | Sim |
