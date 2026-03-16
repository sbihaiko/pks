# PKS v2 — Guia de Implementação para Desenvolvedores Rust

**Data:** 2026-03-16
**Público:** Desenvolvedor Rust iniciando a implementação do PKS v2
**Pré-requisito:** Leitura do `00_plano_geral_v2.md` para contexto estratégico

---

## 1. Grafo de Dependências

```
                    ┌─────────────────────────────┐
                    │  M10 — Singleton Daemon IPC  │
                    │  (tokio, serde_json, fs2,    │
                    │   ctrlc)                     │
                    └──────────────┬───────────────┘
                                  │
                    ┌─────────────▼───────────────┐
                    │  M11 — RepoId + Bare Commits │
                    │  (git2; tempfile[dev])       │
                    └──┬──────────────────────┬───┘
                       │                      │
          ┌────────────▼──────────┐  ┌────────▼─────────────────────┐
          │  M12 — Shadow Journal │  │  M13 — Ollama Opcional +     │
          │  (chrono, uuid)       │  │        pks_execute            │
          └────────────┬──────────┘  └────────┬─────────────────────┘
                       │                      │
                    ┌──▼──────────────────────▼───┐
                    │  M14 — Zero-Config Onboarding│
                    │  (toml — para config.toml)   │
                    └─────────────────────────────┘
```

**Regra:** Nunca iniciar um milestone antes que suas dependências estejam com testes passando.

---

## 2. Sequência de Execução Recomendada

| Ordem | Milestone | Estimativa | Paralelo com |
|-------|-----------|------------|--------------|
| 1     | **M10** — Singleton Daemon + IPC | >7 dias (G) | — |
| 2     | **M11** — RepoId + Bare Commits | 3–7 dias (M) | — |
| 3a    | **M12** — Shadow Journaling | 3–7 dias (M) | M13 |
| 3b    | **M13** — Ollama Opcional + pks_execute | >7 dias (G) | M12 |
| 4     | **M14** — Zero-Config Onboarding | ≤3 dias (P) | — |

M12 e M13 podem ser desenvolvidos em paralelo por devs diferentes, pois ambos dependem apenas de M11.

---

## 3. Master Crate List (Cargo.toml)

Todas as dependências externas necessárias ao longo dos 5 milestones:

```toml
[dependencies]
# M10 — IPC Singleton
tokio = { version = "1.38", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
ctrlc = "3.4"
fs2 = "0.4"

# M11 — RepoId + Bare Commits
git2 = "0.18"

# M12 — Shadow Journaling
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4"] }

# M14 — Zero-Config Onboarding
toml = "0.8"       # para gerar .pks/config.toml

[dev-dependencies]
tempfile = "3.8"    # M11 — repos temporários em testes
```

> **Nota:** A crate `notify` (FSWatcher) deve ser **removida** em M10 (T10.6), conforme STEERING_remove_fswatcher.

---

## 4. Novos Arquivos por Milestone

### M10 — Singleton Daemon + IPC
| Arquivo | Tipo |
|---------|------|
| `pks/src/ipc/mod.rs` | NEW — IpcClient, IpcServer, PksCommand, PksResponse |
| `pks/src/commands/refresh.rs` | NEW — subcomando `pks refresh` |
| `pks/src/cli.rs` | MODIFY — flag `--daemon`, subcomando `refresh` |
| `pks/src/main.rs` | MODIFY — startup com detecção de instância, PID lockfile |
| `pks/deploy/pks.plist` | MODIFY — usar `--daemon` |
| `pks/deploy/pks.service` | MODIFY — usar `--daemon` |
| `pks/tests/singleton_ipc_test.rs` | NEW — teste de integração |

### M11 — RepoId Unificado + Bare Commits
| Arquivo | Tipo |
|---------|------|
| `pks/src/git/repo_identity.rs` | NEW — RepoIdentity, from_path(), is_same_repo() |
| `pks/src/git/bare_commit.rs` | NEW — BareCommit, write_file(), ensure_branch() |
| `pks/src/state.rs` | MODIFY — HashMap key de PathBuf para RepoId |
| `pks/src/git/journal.rs` | MODIFY — substituir checkout por BareCommit |
| `pks/tests/test_repo_identity.rs` | NEW |
| `pks/tests/test_bare_commit.rs` | NEW |

### M12 — Shadow Journaling Passivo
| Arquivo | Tipo |
|---------|------|
| `pks/src/hooks/shadow_journal.rs` | NEW — ShadowJournalHook, JournalEntry |
| `pks/src/ipc/mod.rs` | MODIFY — adicionar PksCommand::RecordToolEvent |
| `pks/tests/shadow_journal_e2e.rs` | NEW |

### M13 — Ollama Opcional + pks_execute
| Arquivo | Tipo |
|---------|------|
| `pks/src/daemon/startup.rs` | MODIFY — PKS_EMBEDDING_PROVIDER |
| `pks/src/embeddings/pipeline.rs` | MODIFY — skip se provider = none |
| `pks/src/mcp/tools/pks_execute.rs` | NEW — PksExecuteTool |
| `pks/src/mcp/server.rs` | MODIFY — registrar pks_execute |
| `.agent/workflows/pks-install.md` | MODIFY — Ollama opcional |
| `pks/tests/pks_execute_integration_test.rs` | NEW |

### M14 — Zero-Config Onboarding
| Arquivo | Tipo |
|---------|------|
| `pks/src/cli/init.rs` | NEW — InitCommand |
| `.agent/workflows/pks-init.md` | NEW — slash command |
| `pks/tests/test_init_e2e.rs` | NEW |

---

## 5. Master Task Checklist (Ordem de Execução)

### Fase 1: M10 — Singleton Daemon + IPC

- [ ] **T10.1** — Refatorar `cli.rs`: flag `--daemon`, separar cliente/servidor
- [ ] **T10.2** — Implementar `ipc/mod.rs`: IpcClient, IpcServer, PksCommand/PksResponse
- [ ] **T10.3** — Atualizar `main.rs`: detecção de instância, PID lockfile, backoff
- [ ] **T10.4** — Atualizar arquivos de serviço (launchd/systemd) para `--daemon`
- [ ] **T10.5** — Teste de integração: singleton e comunicação cliente-servidor
- [ ] **T10.6** — Remover FSWatcher (`notify`) de `main.rs` e `Cargo.toml`
- [ ] **T10.7** — Implementar `pks refresh`: scan de vaults, flags `--dry-run`

**Gate:** `cargo test --test singleton_ipc_test` verde antes de avançar.

### Fase 2: M11 — RepoId Unificado + Bare Commits

- [ ] **T11.1** — Implementar `RepoIdentity`: from_path(), is_same_repo()
- [ ] **T11.2** — Atualizar `PrevalentState`: HashMap<RepoId, RepoIndex>
- [ ] **T11.3** — Implementar `BareCommit`: write_file(), ensure_branch()
- [ ] **T11.4** — Substituir lógica de commit em journal.rs por BareCommit
- [ ] **T11.5** — Teste: duas worktrees → mesmo RepoId
- [ ] **T11.6** — Teste: BareCommit não suja working tree

**Gate:** `cargo test test_repo_identity test_bare_commit` verde antes de avançar.

### Fase 3a: M12 — Shadow Journaling (paralelo com M13)

- [ ] **T12.1** — Implementar struct `JournalEntry` + Serialize
- [ ] **T12.2** — Implementar `record_tool_event()` (zero I/O, só Vec push)
- [ ] **T12.3** — Implementar `render_journal_md()` → Markdown
- [ ] **T12.4** — Implementar `flush_to_vault()` via BareCommit
- [ ] **T12.5** — Adicionar `PksCommand::RecordToolEvent` ao IPC
- [ ] **T12.6** — Teste e2e: 5 eventos → flush → verificar branch

### Fase 3b: M13 — Ollama Opcional + pks_execute (paralelo com M12)

- [ ] **T13.1** — Leitura de `PKS_EMBEDDING_PROVIDER` na inicialização
- [ ] **T13.2** — Pipeline de embeddings condicional (skip se none)
- [ ] **T13.3** — Atualizar `pks-install.md`: Ollama como passo opcional
- [ ] **T13.4** — Implementar ExecuteParams + ExecuteResponse
- [ ] **T13.5** — Implementar `PksExecuteTool::execute()` com sandbox
- [ ] **T13.6** — Registrar `pks_execute` como ferramenta MCP
- [ ] **T13.7** — Teste de integração: executar shell, verificar summary

**Gate (M12+M13):** Todos os testes de ambos passando antes de M14.

### Fase 4: M14 — Zero-Config Onboarding

- [ ] **T14.1** — Implementar `InitCommand` struct com `run()`
- [ ] **T14.2** — Detecção do git root (show-toplevel + git-common-dir)
- [ ] **T14.3** — Geração do template `config.toml`
- [ ] **T14.4** — Criação da branch `pks-knowledge` (órfã, idempotente)
- [ ] **T14.5** — Registro via IPC: `PksCommand::RegisterRepo`
- [ ] **T14.6** — Disparo de indexação + resumo
- [ ] **T14.7** — Criar slash command `/pks-init`
- [ ] **T14.8** — Teste e2e: repo temporário, `pks init`, verificar tudo
- [ ] **T14.9** — Chamar `pks refresh` ao final do init

---

## 6. Quick Start — Primeiro Dia

```bash
# 1. Verificar pré-requisitos
rustc --version          # ≥ 1.77
cargo test --workspace   # deve estar verde (M8/M9 completos)

# 2. Ler o plano geral
cat docs/v2/00_plano_geral_v2.md

# 3. Começar pelo M10 (obrigatório — é a base de tudo)
cat docs/v2/M10_singleton_daemon_ipc.md

# 4. Adicionar as crates do M10 ao Cargo.toml
# tokio, serde, serde_json, ctrlc, fs2

# 5. Primeira tarefa: T10.1 — refatorar cli.rs
# Adicionar flag --daemon e separar modo cliente/servidor

# 6. Validar com o teste de integração (T10.5)
cargo test --test singleton_ipc_test
```

### Checklist de Ambiente

- [ ] Rust toolchain ≥ 1.77 instalado
- [ ] `cargo test --workspace` verde no branch `main`
- [ ] `pks --stdio` funcional com pelo menos um vault
- [ ] Vault com ≥50 arquivos Markdown para testes
- [ ] Nenhum issue aberto com label `blocking`
- [ ] `pks/src/cli.rs` revisado e comportamento atual documentado

---

## 7. Variáveis de Ambiente Introduzidas na v2

| Variável | Default | Milestone | Descrição |
|----------|---------|-----------|-----------|
| `PKS_EMBEDDING_PROVIDER` | `none` | M13 | `none` / `ollama` / `mlx` |
| `PKS_SHADOW_JOURNAL` | `true` | M12 | Habilita/desabilita journaling |
| `PKS_JOURNAL_MIN_WORDS` | `10` | M12 | Mínimo de palavras por sessão |
| `PKS_JOURNAL_MAX_ENTRIES` | `500` | M12 | Máximo de entradas por sessão |
| `PKS_JOURNAL_TRUNCATE` | `200` | M12 | Limite de chars no input summary |
| `PKS_EXECUTE_TOP_N` | `5` | M13 | Chunks no summary do pks_execute |
| `PKS_IPC_VER` | `2` | M10 | Versão do protocolo IPC |

---

## 8. Referência Rápida dos Documentos

| Arquivo | Conteúdo | Linhas |
|---------|----------|--------|
| `00_plano_geral_v2.md` | Visão geral, steerings, dependências, critérios de entrada/conclusão | 96 |
| `M10_singleton_daemon_ipc.md` | IPC, Unix Socket, PID lockfile, auto-spawn, `pks refresh` | 160 |
| `M11_identidade_repo_bare_commits.md` | RepoIdentity, BareCommit, migração de PrevalentState | 311 |
| `M12_shadow_journaling_passivo.md` | JournalEntry, flush via BareCommit, privacidade | 278 |
| `M13_ollama_opcional_context_mode.md` | PKS_EMBEDDING_PROVIDER, pks_execute, sandbox | 282 |
| `M14_zero_config_onboarding.md` | `pks init`, config.toml template, edge cases | 189 |

---

## 9. Dicas Arquiteturais (consolidadas das Observações Críticas)

1. **Deadlock no Startup (M10):** O daemon deve estar pronto no socket ANTES de indexações pesadas. Use um worker thread separado para indexação.
2. **Versioning IPC (M10):** Use `PKS_IPC_VER=2` no protocolo para evitar quebras em upgrades.
3. **Debouncing de Indexação (M11):** Várias worktrees podem disparar indexação simultânea. Implementar serialização por RepoId no Daemon.
4. **Secrets no Journal (M12):** Regex para detecção de secrets é falho. Tratar a branch `pks-knowledge` como sensível.
5. **Eviction de Memória (M13):** Adicionar política LRU para descarregar `tantivy::Index` inativos do daemon.
6. **UX de Ferramentas (M13):** Diferenciar claramente `Bash` vs `pks_execute` na descrição MCP.
