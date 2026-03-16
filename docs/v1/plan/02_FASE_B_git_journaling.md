# Fase B — Git Journaling & Vaults (Milestone M4)

> Objetivo: Conectar o motor PKS ao Git nativo dos repositorios, implementar a estrutura de Vault por projeto, branch `pks-knowledge`, deteccao automatica de mudancas, e monitoramento de repositorios remotos.
> Pre-requisito: Fase A (M1-M3) completa.

---

## Milestone M4: Conexao com o Hospedeiro

### Objetivo
Integrar o PKS Daemon ao ciclo de vida Git dos repositorios: estrutura de vault padrao, branch dedicado para conhecimento, hooks de deteccao, reindexacao automatica, e monitoramento de repositorios remotos.

### Tasks

#### T4.1 — Estrutura do Vault por Projeto (F1.1) ✅ CONCLUIDA
- **Goal**: Criar tooling para inicializar a estrutura `prometheus/` dentro de qualquer repo Git
- **Files**: `[CREATE] pks/src/vault_init.rs`, `[CREATE] pks/src/cli.rs` (comando `pks init`)
- **Tests**: Teste: `pks init` em repo vazio, verificar hierarquia criada
- **Acceptance**:
  - [x] Comando `pks init` cria estrutura: `prometheus/{01-domains, 02-features, 03-testing, 04-workflows, 05-decisions, 90-ai-memory}`
  - [x] Estrutura e sugestao — indexador funciona com qualquer hierarquia de `.md`
  - [x] Teste: indexar vault com estrutura Zettelkasten (flat files, sem pastas 01-06) — indexador funciona corretamente. Teste: indexar vault com estrutura customizada (nomes arbitrarios) — indexador funciona. Principio "Convencao, nao contrato" verificavel por teste
  - [x] `.obsidian/` criado se nao existir. Config basica inclui: `workspace.json` (layout default), `.obsidian/app.json` (configuracoes de editor). Suficiente para abrir vault sem erros
  - [x] Idempotente: rodar 2x nao duplica nada
  - [x] Abre no Obsidian sem erros e com navegacao funcional entre notas via wikilinks

#### T4.2 — Branch `pks-knowledge` + Git Worktree (F1.2) ✅ CONCLUIDA
- **Goal**: Criar e gerenciar branch dedicado via `git worktree` para isolar commits de conhecimento
- **Files**: `[CREATE] pks/src/git_branch.rs`, `[MODIFY] pks/src/vault_init.rs`
- **Tests**: Teste: inicializar worktree, commitar nota, verificar que `main` nao e poluido
- **Acceptance**:
  - [x] `pks init` cria branch orfao `pks-knowledge` automaticamente
  - [x] Nota de design: branch orfao escolhido para evitar ancestralidade com `main`, garantindo que `git log main` nunca mostra commits de conhecimento. Alternativa descartada: branch normal a partir de main — poluiria merge-base e dificultaria rebase
  - [x] `prometheus/` configurado como worktree atrelado a `pks-knowledge`
  - [x] `prometheus/` adicionado ao `.git/info/exclude` (nao `.gitignore`)
  - [x] IDEs (VS Code) continuam indexando `prometheus/` nas buscas
  - [x] Commits automaticos de import/resumo vao para `pks-knowledge`
  - [x] Commit Linking: SHA de `main` injetado nos metadados YAML. Campo YAML: `source_commit_sha` (consistente com frontmatter de F1.3/T6.1)
  - [x] Decisao D13 (PRD): worktree sacrifica atomicidade (commit unico codigo+nota impossivel) em troca de historico limpo em `main`. Trade-off consciente: rastreabilidade bidirecional via SHA no frontmatter YAML compensa a perda de atomicidade. Commit Linking (SHA de `main` -> metadados de `pks-knowledge`) e o mecanismo de reconciliacao
- **MCP Reference**: Estudar `server-git` para patterns de operacao Git via MCP

#### T4.2b — CLI `pks doctor` (Diagnostico e Reparo) ✅ CONCLUIDA
- **Goal**: Implementar comando `pks doctor <path>` para diagnosticar e reparar estados degradados do setup PKS conforme PRD F1.2 (linhas 134-137)
- **Files**: `[MODIFY] pks/src/cli.rs`, `[CREATE] pks/src/doctor.rs`
- **Tests**: Teste: corromper worktree manualmente, rodar `pks doctor`, verificar reparo
- **Acceptance**:
  - [x] `pks doctor <path>` verifica:
    - Worktree existe e aponta para o branch correto (`pks-knowledge`)?
    - `.git/info/exclude` contem `prometheus/`?
    - Hook post-commit instalado e executavel?
    - Branch `pks-knowledge` existe localmente e no remote?
  - [x] Para cada problema detectado, oferece reparo automatico interativo
  - [x] Output formatado com status por verificacao (OK / WARN / ERROR)
  - [x] Exit code: 0 = tudo ok, 1 = problemas encontrados (com ou sem reparo)
  - [x] Reparos suportados: recriar worktree, atualizar exclude, reinstalar hook, criar branch local a partir do remote
  - [x] Complementa `pks init` — init cria do zero, doctor repara estado existente

#### T4.3 — Visibilidade Hibrida e Resolucao de Conflitos (F1.2) ✅ CONCLUIDA
- **Goal**: Resolver conflitos de co-edicao (Obsidian vs automacao) via Amortizacao Destrutiva
- **Files**: `[CREATE] pks/src/conflict_resolver.rs`
- **Tests**: Teste: simular edicao simultanea Obsidian + commit automatico, verificar resolucao
- **Acceptance**:
  - [x] Conflitos resolvidos otimisticamente (AP > CP)
  - [x] Ultima versao viavel priorizada
  - [x] Commit problematico descartado silenciosamente se necessario
  - [x] Log estruturado de cada resolucao
  - [x] Sem merge conflicts bloqueantes para o usuario

#### T4.4 — Deteccao de Mudancas: Hook + FS Events (F2.1) ✅ CONCLUIDA
- **Goal**: Implementar hierarquia de deteccao: post-commit hook (primario) + OS FS Events (fallback) + repo_watcher (registro)
- **Files**: `[CREATE] pks/src/git_journal.rs`, `[MODIFY] pks/src/debounce.rs` (criado provisoriamente em T2.2/Fase A; aqui estendido com hooks)
- **Tests**: Teste: commit via CLI, verificar deteccao; pull externo, verificar FS Event
- **Acceptance**:
  - [x] Hierarquia unificada de deteccao integra 3 mecanismos: (1) Post-commit hook (primario), (2) OS FS Events em `.git/refs/heads/` (fallback), (3) FS watch de `{PKS_VAULTS_DIR}` via `repo_watcher` (T1.4/Fase A) para registro/desregistro de repos. Os 3 mecanismos alimentam a mesma Fila 1 do pipeline
  - [x] Post-commit hook instalado automaticamente pelo `pks init`
  - [x] OS FS Events via `fsevents` (macOS) / `inotify` (Linux) em `.git/refs/heads/`
  - [x] Debounce: mesmo `Commit SHA` **ou** `Tree Hash` em janela curta = 1 entrada na Fila 1 (conforme PRD F2.1 — cobre amend/rewrite com arvore identica). Janela de debounce default: 500ms. Configuravel via `PKS_DEBOUNCE_WINDOW_MS`
  - [x] Backpressure: se Fila 1 estiver saturada (PKS_FILA1_MAX), hooks e FS events sao aceitos mas transacoes excedentes descartadas com tolerancia a perda (recuperaveis via Vector Clock conforme T3.2). Log WARN emitido
  - [x] Captura: commit, pull, merge, rebase, amend
  - [x] Daemon acompanha HEADs de branches relevantes (`main`, `feature/*`, `pks-knowledge`)
  - [x] Granularidade atomica: commit com 50 arquivos = 1 transacao
  - [x] Medicao via tracing timestamps: intervalo entre commit e entrada na Fila 1 < 100ms
  - [x] Nota: processamento de commits via hooks/FS events alimenta pipeline (Fase A) que por sua vez dispara snapshots periodicos conforme T3.1
- **MCP Reference**: Usar `git2-rs` para diff parsing

#### T4.5 — Reindexacao pos-Rebase (F2.5) ✅ CONCLUIDA
- **Goal**: Detectar rebase/amend e disparar reindex completo do repositorio afetado
- **Files**: `[MODIFY] pks/src/git_journal.rs`, `[MODIFY] pks/src/state.rs`
- **Tests**: Teste: fazer `git rebase -i`, verificar que indice e reconstruido
- **Acceptance**:
  - [x] Rebase detectado via mudanca de SHA na HEAD sem parentesco linear
  - [x] Branch checkout (`git checkout <branch>`): detectado via mudanca na HEAD ref sem novo commit. Daemon invalida state do repo e recarrega da nova HEAD. Teste: checkout entre branches com arquivos .md diferentes, verificar que indice reflete a nova branch
  - [x] Drop & Rebuild: particao do repo ejetada e reindexada do zero
  - [x] BM25 reindexado imediatamente
  - [x] Vetores reindexados assincronamente via Fila 1
  - [x] Target de performance: Drop & Rebuild BM25 de repo com 1000 notas < 30s. Vetores convergem assincronamente conforme pipeline
  - [x] Vector Clock atualizado apos reconstrucao
  - [x] Tambem cobre force-push remoto: non-fast-forward detectado em T4.6 reutiliza esta mesma logica de Drop & Rebuild

#### T4.6 — Monitoramento de Repositorios Remotos (F2.7) ✅ CONCLUIDA
- **Goal**: Suportar repos remotos via clone automatico (SSH/HTTPS) em `{PKS_VAULTS_DIR}`, usando polling puro via `git fetch`
- **Files**: `[CREATE] pks/src/remote_sync.rs`, `[MODIFY] pks/src/repo_watcher.rs`
- **Tests**: Teste: configurar repo remoto, verificar clone e indexacao automatica
- **Acceptance**:
  - [x] Suporte a clone via SSH e HTTPS para `{PKS_VAULTS_DIR}`
  - [x] Polling configuravel para fetch de atualizacoes remotas (ex: `PKS_REMOTE_POLL_INTERVAL=300s`)
  - [x] `git fetch` + deteccao de novos commits via comparacao de refs
  - [x] Cross-pollination: repo remoto indexado na mesma RAM que repos locais
  - [x] Falha de rede nao impacta repos locais (isolamento de particao — AP do CAP)
  - [x] Credenciais via `GITHUB_TOKEN` env var (ja listado no plano geral)
  - [x] Nota: integracao via API GitHub/GitLab (webhooks, API de refs) descartada nesta fase em favor de polling puro via `git fetch`. Justificativa: polling e mais simples, funciona com qualquer host Git, e nao requer configuracao de webhooks. API route documentada como extensao futura
  - [x] **F2.8 Partition Tolerance**: falha de rede em repo remoto nao impacta queries de repos locais; teste de integracao simula timeout de fetch e verifica que daemon continua respondendo normalmente
  - [x] Cenarios extremos de particao (pushes conflitantes, degeneracao de branch pointer) tratados com ejecao silenciosa da particao afetada + Drop & Rebuild (reutiliza T4.5). Log WARN emitido. Teste de integracao: simular ref corrompida apos fetch, verificar ejecao e rebuild
  - [x] Non-fast-forward remote update (`git push --force` no remoto): detectado via comparacao de refs pos-fetch, dispara Drop & Rebuild da particao remota (reutiliza logica de T4.5)
- **MCP Reference**: Estudar `server-git` para patterns de operacao Git remota

#### T4.7 — Memoria Reflexiva Global (F2.6) ✅ CONCLUIDA
- **Goal**: Sumarizar decisoes da sessao AI em `prometheus/90-ai-memory/YYYY-MM-DD.md`
- **Files**: `[CREATE] pks/src/memory_writer.rs`
- **Tests**: Teste: gerar resumo, verificar commit em `pks-knowledge` com SHA reference
- **Acceptance**:
  - [x] Trigger de geracao: chamada explicita via MCP tool `pks_session_summary(session_context)` pelo Antigravity ao encerrar sessao. Alternativa: hook de shell (`EXIT` trap) que invoca CLI `pks summarize-session`. O PKS nao detecta automaticamente o encerramento — depende de invocacao explicita
  - [x] Resumo gerado no encerramento da sessao Antigravity
  - [x] Commitado automaticamente em `pks-knowledge`
  - [x] Metadados YAML incluem `session_sha` (ultimo commit de trabalho da sessao)
  - [x] Template do resumo: frontmatter YAML (`session_sha`, `date`, `topics[]`, `repos_touched[]`) + secoes Markdown (`## Decisoes`, `## Contexto`, `## Proximos Passos`)
  - [x] Rastreabilidade bidirecional: codigo <-> contexto
  - [x] Historico viaja com `git clone`/`git fetch`
  - [x] Conteudo do resumo segue politica de armazenamento seletivo (T6.5/F1.5): guarda contexto e raciocinio, nao transcricao bruta da sessao

### Criterios de Sucesso M4
- [x] `pks init` configura vault + branch + worktree em um comando
- [x] Commits em `main` detectados em < 100ms e enfileirados no Daemon
- [x] Atualizacao BM25 pos-commit: sub-100ms (atomico, single-thread) conforme PRD
- [x] Pull/merge/rebase detectados via FS Events sem polling
- [x] Debounce elimina double-firing (1 evento por commit)
- [x] Rebase nao corrompe — rebuild automatico verificado por teste
- [x] Obsidian abre vault do worktree sem conflitos com workspace de codigo
- [x] Historico de `main` permanece limpo (zero commits de notas/resumos)
- [x] Repos remotos clonados e indexados automaticamente via T4.6
- [x] Falha de rede em repo remoto nao impacta repos locais
- [x] `pks doctor` detecta e repara worktree corrompido, exclude ausente, hook faltando
- [x] Teste: query retorna resultados tanto de `main` (codigo) quanto de `pks-knowledge` (notas) simultaneamente — indice unificado na RAM conforme PRD F1.2

---

## Dependencias Externas (Fase B)

| Dependencia | Instalacao | Obrigatoria? |
|-------------|-----------|--------------|
| git2-rs | Cargo dependency | Sim |
| Obsidian | Download manual | Recomendado (validacao visual) |
| Repo Git de teste com codigo | Qualquer projeto real | Sim |

---

## Riscos e Mitigacoes

### Riscos de Implementacao

| Risco | Impacto | Mitigacao |
|-------|---------|-----------|
| Worktree instavel em repos grandes | Setup falha | Testar com repos de 1k+ arquivos; fallback para branch tracking sem worktree |
| `.git/info/exclude` ignorado por alguma IDE | prometheus/ aparece no git status | Documentar e oferecer fallback `.gitignore` com flag |
| FS Events nao disparam em todos os OS | Mudancas nao detectadas | Polling de baixa frequencia como ultimo fallback (30s) |
| Rebase em branch com muitos commits | Reindex lento | Limitar profundidade de reindex; usar snapshot anterior como base |
| `git2-rs` lento em repos com milhares de commits | Drop & Rebuild demorado | Limitar profundidade de diff parsing; usar `--shallow` para repos remotos grandes |

### Riscos Operacionais (PRD)

| Risco | Impacto | Mitigacao |
|-------|---------|-----------|
| Worktree orfa (git clean -fdx) | CI ou tools destroem worktree silenciosamente | `pks doctor` detecta e repara automaticamente (T4.2b) |
| Interacao com Obsidian file watcher | Reset do branch pode corromper `.obsidian/` | `pks init` configura `.obsidian/` no `.gitignore` interno do branch `pks-knowledge` |
| Onboarding (git clone nao traz worktree) | Dev novo nao tem `prometheus/` | `pks init` obrigatorio apos clone (documentado no README) |
| GUI Git clients confusos com worktrees | GitKraken, SourceTree, VS Code Git panel | Risco aceito; worktrees sao padrao Git oficial desde v2.5 |
| `.git/info/exclude` nao viaja com clone | Cada maquina nova precisa reconfigurar | `pks init` configura automaticamente |
