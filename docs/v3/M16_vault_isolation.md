# M16 — Vault Isolation

| Campo | Valor |
|---|---|
| **Status** | PLANEJADO |
| **Depende de** | M11 (RepoIdentity), M14 (pks init) |
| **Complexidade** | Pequeno |
| **Arquivos principais** | `src/boot_indexer.rs` |

---

## 1. Diagnóstico

O walker do `boot_indexer.rs` não exclui `prometheus/` ao varrer um repositório:

```rust
// boot_indexer.rs:14 — lista de exclusão atual
if name.starts_with('.') || name == "node_modules" || name == "target"
    || name == "vendor" || name == "venv" || name == ".venv" {
    return;
}
// "prometheus" não está aqui → é varrido junto com o repo pai
```

Consequência: arquivos `.md` em `prometheus/` são indexados com o `repo_id` do projeto pai. Journals de sessão, decisões e notas do vault ficam misturados com documentação técnica do projeto no mesmo índice — impossibilitando filtros por origem.

O `prometheus/` é um git worktree — tem um arquivo `.git` (pointer), não um diretório `.git`. O `RepoWatcher::is_git_repo()` verifica `path.join(".git").exists()`, o que passa para arquivos também. No entanto, o `scan_existing_repos()` só varre o primeiro nível de `PKS_VAULTS_DIR`, não entra em subdirs — então `prometheus/` nunca é registrado como repo independente.

---

## 2. Solução

Duas mudanças independentes e complementares:

### 2.1 Excluir o Vault do Walker (boot_indexer)

Usar uma constante centralizada para o nome do vault, evitando strings soltas no meio do walker:

```rust
// constants.rs (ou boot_indexer.rs)
pub const VAULT_DIR_NAME: &str = "prometheus";
```

```rust
// boot_indexer.rs — collect_md_entry()
use crate::constants::VAULT_DIR_NAME;
if name.starts_with('.') || name == "node_modules" || name == "target"
    || name == "vendor" || name == "venv" || name == ".venv"
    || name == VAULT_DIR_NAME {
    return;
}
```

**Nota sobre o `repo_watcher.rs`:** O watcher atual é apenas um scanner de boot (`scan_existing_repos()`), sem listener de filesystem reativo. Não há `Notify` nem loop de eventos. Logo, a exclusão no `boot_indexer` é suficiente — não há rota secundária de ingestão ao vivo que precise ser protegida.

A pesquisabilidade imediata dos journals recém-commitados é resolvida no M15 via **re-indexação inline** no próprio `submit-journal` / `flush-session`, sem necessidade de watcher reativo.

### 2.2 Registrar `prometheus/` como vault independente

O `scan_existing_repos()` hoje usa `read_dir()` com profundidade 1. Estender para detectar worktrees dentro dos repos encontrados:

**Opção A (simples):** Após indexar um repo, verificar se `{repo_root}/prometheus/.git` existe (arquivo, não diretório) e registrá-lo como vault adicional.

**Opção B (genérica):** Adicionar suporte a worktrees na descoberta de repos, listando `git worktree list` para cada repo encontrado.

**Recomendação: Opção A** — escopo menor, sem dependência de CLI git na inicialização.

```rust
// boot_indexer.rs — após index_repo()
pub async fn index_prometheus_worktree(
    repo_path: &Path,
    pipeline: &mut IndexingPipeline,
    state: &Arc<Mutex<PrevalentState>>,
) {
    let prometheus = repo_path.join("prometheus");
    // Arquivo .git indica worktree
    if prometheus.join(".git").is_file() {
        // repo_id próprio: nome do diretório + sufixo vault
        let repo_id = format!(
            "{}-vault",
            repo_path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
        );
        let mut paths = Vec::new();
        walk_md_files(&prometheus, &mut paths);
        for file_path in &paths {
            ingest_file_chunks(&repo_id, file_path, pipeline, state).await;
        }
    }
}
```

---

## 3. Subtarefas

| ID | Tarefa | Arquivo | Depende de |
|---|---|---|---|
| T16.1 | Criar constante `VAULT_DIR_NAME` e usá-la no walker de exclusão | `src/boot_indexer.rs` | — |
| T16.2 | Implementar `index_vault_worktree()` — indexa o worktree com `repo_id` próprio (`{repo}-vault`) | `src/boot_indexer.rs` | T16.1 |
| T16.3 | Chamar `index_vault_worktree()` em `index_vaults_on_boot()` para cada repo encontrado | `src/boot_indexer.rs` | T16.2 |
| T16.4 | Testes: walker não indexa `prometheus/` junto com o repo pai | `src/boot_indexer.rs` (tests) | T16.1 |
| T16.5 | Testes: `prometheus/` indexado com `repo_id` separado (`{repo}-vault`) | `src/boot_indexer.rs` (tests) | T16.2 |

---

## 4. Critérios de Aceite

- [ ] Arquivos em `prometheus/` nunca aparecem no `repo_id` do projeto pai no índice Tantivy
- [ ] `prometheus/journals/*.md` são indexados e pesquisáveis com `repo_id = "{projeto}-vault"`
- [ ] `pks search "decisão"` retorna resultados de `prometheus/` identificados pelo `repo_id` vault
- [ ] `cargo test --workspace` passa após as mudanças
- [ ] Nenhuma regressão em repos sem `prometheus/` worktree
