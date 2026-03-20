# M17 — Simplificação do Prometheus + Gatilhos Universais

| Campo | Valor |
|---|---|
| **Status** | PLANEJADO |
| **Depende de** | M15 (Shadow Journaling), M16 (Vault Isolation) |
| **Complexidade** | Médio |
| **Arquivos principais** | `src/vault_init.rs`, `src/cli/mod.rs`, `src/cli/decision.rs` [NEW], `src/mcp_server.rs`, `src/tracker/import/routing.rs`, `src/memory_writer.rs`, `src/git_journal_append.rs` |

---

## 1. Motivação

A estrutura atual do `prometheus/` contém 6 pastas numeradas (`01-domains`, `02-features`, `03-testing`, `04-workflows`, `05-decisions`, `90-ai-memory`), das quais apenas 2 possuem gatilhos reais de escrita (`02-features` via tracker import e `90-ai-memory` via session summary). As demais são apenas criadas pelo `pks init` e nunca recebem dados automaticamente — ficam vazias indefinidamente.

**Decisão:** Reduzir para **3 pastas funcionais**, todas com gatilhos de escrita (CLI + MCP), eliminando pastas mortas e tornando os nomes intuitivos.

---

## 2. Nova Estrutura

| Pasta | Gatilho CLI | Gatilho MCP | Propósito |
| :--- | :--- | :--- | :--- |
| `features/` | `pks tracker import` | `pks_add_feature` | Requisitos e specs de tickets |
| `decisions/` | `pks decision <msg>` | `pks_add_decision` | ADRs e decisões técnicas |
| `journals/` | `pks flush-session` / `pks submit-journal` | `pks_session_summary` | Logs de sessão e contexto |

### O que foi absorvido/removido:
- `01-domains` → Absorvido por `decisions/` (definições de domínio são decisões arquiteturais)
- `03-testing` → Removido (sem caso de uso validado)
- `04-workflows` → Removido (sem caso de uso validado)
- `05-decisions` → Renomeado para `decisions/`
- `90-ai-memory` → Renomeado para `journals/` (convergindo com o path já usado pelo `shadow_journal.rs`)

---

## 3. Decisões Técnicas

### 3.1 Resolução de `repo_path`
- **Convenção:** `PKS_VAULTS_DIR` (env var, default `$HOME/pks-vaults`) define o diretório pai de todos os projetos.
- **CLI:** infere a partir de `cwd` (padrão de `submit_journal.rs`).
- **MCP:** recebe `repo_id` como parâmetro. O handler resolve para o path absoluto via `RepoWatcher::scan_existing_repos()`.
- **Caminhos internos:** sempre relativos ao repo (ex: `decisions/2026-03-20_abc.md`).

### 3.2 Formato ADR (`decisions/`)
```markdown
---
date: 2026-03-20T10:30:00Z
source: cli | mcp
context: <opcional>
---
# <título da decisão>

<conteúdo livre>
```

### 3.3 Formato Feature (`features/`)
```markdown
---
date: 2026-03-20T10:30:00Z
source: tracker | mcp
tracker_id: <opcional, ex: "PAY-4421">
---
# <título>

<conteúdo livre>
```

---

## 4. Mudanças no Código

### 4.1 Inicialização

**`src/vault_init.rs`:**
```rust
const PROMETHEUS_DIRS: &[&str] = &[
    "prometheus/features",
    "prometheus/decisions",
    "prometheus/journals",
];
```

### 4.2 Novo Comando CLI: `pks decision`

**`src/cli/mod.rs`** — adicionar variante:
```rust
pub enum CliCommand {
    // ... existentes ...
    Decision { note: String },
}
```

**`src/cli/decision.rs`** [NEW]:
```rust
pub fn run_decision(note: &str) -> i32 {
    // 1. cwd → repo root
    // 2. Gera frontmatter YAML (date, source: "cli")
    // 3. Filename: decisions/YYYY-MM-DD_<hash_8chars>.md
    // 4. BareCommit::write_file na branch pks-knowledge
    // 5. IPC Refresh passivo
    // 6. Exit 0
}
```

### 4.3 Novas Ferramentas MCP

**`src/mcp_server.rs`** — adicionar:

```rust
#[tool(
    name = "pks_add_decision",
    description = "Record an architecture decision (ADR) in the project's knowledge vault."
)]
async fn pks_add_decision(
    &self,
    Parameters(params): Parameters<AddDecisionParams>,
) -> String {
    // repo_id → resolve path via state
    // BareCommit → decisions/YYYY-MM-DD_<hash>.md
}

#[tool(
    name = "pks_add_feature",
    description = "Record a feature specification in the project's knowledge vault."
)]
async fn pks_add_feature(
    &self,
    Parameters(params): Parameters<AddFeatureParams>,
) -> String {
    // repo_id → resolve path via state
    // BareCommit → features/<sanitized_title>.md
}
```

### 4.4 Atualização de Referências

| Arquivo | Mudança |
|---|---|
| `src/tracker/import/routing.rs` | `DEFAULT_DEST_DIR`: `"02-features"` → `"features"` |
| `src/memory_writer.rs` | `"90-ai-memory/{}.md"` → `"journals/{}.md"` |
| `src/git_journal_append.rs` | `"90-ai-memory"` → `"journals"` em `daily_log_path()` |
| `src/doctor.rs` | Validar checks com novas pastas |

---

## 5. Interação com M15 e M16

### Com M15 (Shadow Journaling)
- **Alinhamento:** `shadow_journal.rs` já grava em `journals/` (L88). A renomeação de `90-ai-memory` para `journals/` nos demais arquivos completa a convergência.
- **Sem conflito:** M15 define *como* os journals chegam, M17 define *para onde* vão.

### Com M16 (Vault Isolation)
- **Alinhamento:** `boot_indexer.rs` usa `VAULT_DIR_NAME = "prometheus"` — sem mudança necessária. O M16 garante que `prometheus/` seja excluído do index do repo pai e indexado como vault independente.
- **Benefício adicional:** Com M17, as novas ferramentas MCP geram conteúdo buscável no vault. O M16 garante que esse conteúdo seja encontrado via `{repo}-vault` no search.

---

## 6. Subtarefas

| ID | Tarefa | Arquivo | Depende de |
|---|---|---|---|
| T17.1 | Reduzir `PROMETHEUS_DIRS` para 3 pastas | `src/vault_init.rs` | — |
| T17.2 | Atualizar `DEFAULT_DEST_DIR` para `"features"` | `src/tracker/import/routing.rs` | — |
| T17.3 | Renomear `90-ai-memory` → `journals` em `memory_writer.rs` e `git_journal_append.rs` | `src/memory_writer.rs`, `src/git_journal_append.rs` | — |
| T17.4 | Implementar `CliCommand::Decision` + `run_decision()` | `src/cli/mod.rs`, `src/cli/decision.rs` [NEW] | — |
| T17.5 | Implementar ferramenta MCP `pks_add_decision` | `src/mcp_server.rs` | T17.4 |
| T17.6 | Implementar ferramenta MCP `pks_add_feature` | `src/mcp_server.rs` | T17.2 |
| T17.7 | Validar `doctor.rs` com novas pastas | `src/doctor.rs` | T17.1 |
| T17.8 | Testes para `vault_init`, `routing`, `decision`, ferramentas MCP | múltiplos | T17.1–T17.7 |

---

## 7. Critérios de Aceite

- [ ] `pks init` cria exatamente 3 pastas: `features/`, `decisions/`, `journals/`
- [ ] `pks decision "Usar Rust 2024"` grava ADR em `decisions/` via BareCommit
- [ ] Ferramenta MCP `pks_add_decision` cria ADR buscável via `pks search`
- [ ] Ferramenta MCP `pks_add_feature` cria spec em `features/`
- [ ] `DEFAULT_DEST_DIR` é `"features"` e tracker import funciona
- [ ] Referências a `90-ai-memory` eliminadas — tudo aponta para `journals/`
- [ ] `cargo test --workspace` verde após todas as mudanças
- [ ] Nenhuma regressão nos testes de M15 e M16
