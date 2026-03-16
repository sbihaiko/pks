# M11 — RepoId Unificado + Bare Commits

| Campo        | Valor                              |
|--------------|------------------------------------|
| **Status**   | PENDENTE                           |
| **Depende**  | M10 (Singleton Daemon + IPC)       |
| **Módulos**  | `pks/src/git/repo_identity.rs`, `pks/src/git/bare_commit.rs` |

---

## Objetivo

Resolver dois defeitos estruturais que se manifestam quando o PKS opera sobre repositórios com múltiplas worktrees: a ausência de uma identidade de repositório estável (RepoId) e a contaminação da working tree do desenvolvedor com commits de metadados internos do PKS. O M11 introduz `RepoIdentity` para derivar o RepoId do caminho canônico do diretório `.git` comum, e `BareCommit` para gravar na branch `pks-knowledge` inteiramente via plumbing Git — sem tocar o index ou o working tree ativos.

---

## Problema 1: RepoId Inconsistente com Múltiplas Worktrees

O PKS atualmente usa o CWD (diretório de trabalho corrente) como chave primária do repositório no `PrevalentState`. Isso significa que o mapa em memória é:

```rust
// Estado atual (ERRADO para worktrees)
repos: HashMap<PathBuf, RepoIndex>
// /Users/dev/ProjetoA       → RepoIndex { ... }
// /Users/dev/ProjetoA-feat  → RepoIndex { ... }  ← duplicata!
```

Quando o desenvolvedor cria uma worktree adicional com `git worktree add ../ProjetoA-feat feature/x`, o Git cria um diretório separado que aponta para o mesmo repositório (`--git-common-dir` idêntico), mas o PKS os trata como dois projetos distintos. Consequências:

- **Índice BM25 duplicado**: os mesmos arquivos são indexados duas vezes, consumindo RAM desnecessária.
- **Inconsistência de busca**: uma nota criada no contexto de `/ProjetoA` não é encontrada ao buscar de `/ProjetoA-feat`.
- **Renomear a pasta quebra o histórico**: mover `/ProjetoA` para `/ProjetoA-v2` gera um RepoId novo, perdendo o índice acumulado.

O problema raiz é que `PathBuf` do CWD é uma identidade frágil — ela muda com o sistema de arquivos, mas o repositório Git permanece o mesmo.

---

## Solução: `git rev-parse --git-common-dir`

O Git resolve este problema internamente com o conceito de **git common directory**: o diretório que contém os objetos, refs e configuração compartilhados por todas as worktrees. Em um repositório sem worktrees extras, `--git-common-dir` retorna o próprio `.git/`. Em repositórios com worktrees adicionais, todas as worktrees retornam o mesmo caminho canônico.

```bash
# Worktree principal
$ git -C /Users/dev/ProjetoA rev-parse --git-common-dir
/Users/dev/ProjetoA/.git

# Worktree secundária
$ git -C /Users/dev/ProjetoA-feat rev-parse --git-common-dir
/Users/dev/ProjetoA/.git   # ← mesmo caminho!

# Após renomear a pasta principal para ProjetoA-v2
$ git -C /Users/dev/ProjetoA-v2 rev-parse --git-common-dir
/Users/dev/ProjetoA-v2/.git  # ← caminho atualizado automaticamente pelo Git
```

O RepoId derivado deste caminho — após `std::fs::canonicalize` para resolver symlinks — é invariante a:

- Qual worktree está ativa no momento
- Renomeação da pasta da worktree secundária
- Diferenças de case em sistemas HFS+ (macOS)

A struct proposta em `pks/src/git/repo_identity.rs`:

```rust
pub struct RepoIdentity {
    /// RepoId: PathBuf do git-common-dir canônico. Chave primária no PrevalentState.
    pub repo_id: RepoId,
    /// Caminho retornado por `git rev-parse --git-common-dir`
    pub git_common_dir: PathBuf,
    /// Todas as worktrees ativas (de `git worktree list --porcelain`)
    pub worktrees: Vec<PathBuf>,
}

impl RepoIdentity {
    /// Executa `git -C path rev-parse --git-common-dir` e canonicaliza o resultado.
    /// Retorna erro se o diretório não é um repositório Git válido.
    pub fn from_path(path: &Path) -> Result<Self> { ... }

    /// Compara o git-common-dir de dois caminhos. True se são o mesmo repositório.
    pub fn is_same_repo(a: &Path, b: &Path) -> bool { ... }
}
```

---

## Problema 2: Commits que Sujam a Working Tree

O PKS grava metadados de indexação na branch `pks-knowledge` do repositório monitorado. O fluxo atual usa `git checkout pks-knowledge`, escreve arquivos em disco, faz commit, e retorna ao branch original. Este fluxo tem falhas graves em worktrees secundárias:

1. **`fatal: already checked out`**: o Git rejeita checkout de uma branch que já está ativa em outra worktree. Se `pks-knowledge` está checada em `/ProjetoA`, a tentativa de checkout em `/ProjetoA-feat` falha.
2. **Sujeira no working tree do desenvolvedor**: mesmo quando funciona, o checkout altera o diretório de trabalho visível, podendo interromper compilações ou watchers em execução.
3. **Conflitos com modificações locais**: se o desenvolvedor tem arquivos modificados não commitados, o checkout é bloqueado pelo Git para evitar perda de dados.
4. **Race condition**: entre o checkout e o commit, outro processo pode detectar o estado intermediário inconsistente.

A raiz do problema é usar comandos Git de alto nível (`git checkout`, `git commit`) que operam sobre o working tree e o index — abstrações que existem para o benefício do desenvolvedor humano, não para escrita automatizada de metadados.

---

## Solução: Bare Commits via Git Plumbing

Git expõe comandos de *plumbing* (encanamento interno) que operam diretamente no object store, sem tocar o working tree ou o index. O fluxo completo para gravar um arquivo na branch `pks-knowledge` sem efeitos colaterais:

### Passo 1 — Criar o blob (conteúdo do arquivo)

```bash
echo "conteúdo" | git hash-object -w --stdin
# → a1b2c3d4e5f6...  (SHA-1 do blob gravado no object store)
```

Em Rust via `git2`:
```rust
let blob_oid = repo.blob(content)?;
```

### Passo 2 — Montar a tree (estrutura de diretórios)

Obtém a tree do commit HEAD da branch `pks-knowledge` (se existir) e cria uma nova tree com o arquivo adicionado/atualizado:

```bash
git update-index --add --cacheinfo 100644,<blob-sha>,caminho/arquivo.md
git write-tree
# → f1e2d3c4b5a6...  (SHA-1 da nova tree)
```

Em Rust via `git2`:
```rust
let mut builder = repo.treebuilder(parent_tree.as_ref())?;
builder.insert(filename, blob_oid, 0o100644)?;
let tree_oid = builder.write()?;
```

### Passo 3 — Criar o commit

```bash
git commit-tree <tree-sha> -p <parent-sha> -m "pks: atualiza índice"
# → 9a8b7c6d5e4f...  (SHA-1 do novo commit)
```

Em Rust via `git2`:
```rust
let commit_oid = repo.commit(
    Some("refs/heads/pks-knowledge"),
    &sig, &sig,
    message,
    &tree,
    &[&parent_commit],
)?;
```

### Passo 4 — Atualizar a ref da branch

O `repo.commit` com `Some("refs/heads/pks-knowledge")` já atualiza a ref atomicamente. Nenhum arquivo é criado, modificado ou deletado no working tree.

A struct proposta em `pks/src/git/bare_commit.rs`:

```rust
pub struct BareCommit {
    repo_path: PathBuf,
    branch: String,  // sempre "pks-knowledge"
}

impl BareCommit {
    /// Grava content em path dentro da branch, criando um commit com message.
    /// Não modifica o working tree nem o index visível ao desenvolvedor.
    pub fn write_file(&self, path: &str, content: &[u8], message: &str) -> Result<Oid> { ... }

    /// Verifica se a branch pks-knowledge existe; cria como branch órfã se não existir.
    pub fn ensure_branch(&self) -> Result<()> { ... }
}
```

---

## Subtarefas

| ID     | Descrição                                                                 | Arquivos                              | Status   |
|--------|---------------------------------------------------------------------------|---------------------------------------|----------|
| T11.1  | Implementar `RepoIdentity` — struct, `from_path()`, `is_same_repo()`     | `[NEW] pks/src/git/repo_identity.rs`  | PENDENTE |
| T11.2  | Atualizar `PrevalentState` para usar `RepoId` (git-common-dir) como chave em vez de CWD | `[MODIFY] pks/src/state.rs`           | PENDENTE |
| T11.3  | Implementar `BareCommit` — `write_file()`, `ensure_branch()`             | `[NEW] pks/src/git/bare_commit.rs`    | PENDENTE |
| T11.4  | Substituir lógica atual de commit na `pks-knowledge` por `BareCommit`    | `[MODIFY] pks/src/git/journal.rs`     | PENDENTE |
| T11.5  | Teste: duas worktrees do mesmo repo mapeiam para o mesmo `RepoId`        | `[NEW] pks/tests/test_repo_identity.rs` | PENDENTE |
| T11.6  | Teste: `BareCommit::write_file` grava na `pks-knowledge` sem modificar o working tree | `[NEW] pks/tests/test_bare_commit.rs` | PENDENTE |

### Detalhamento das Subtarefas

#### T11.1 — Implementar `repo_identity.rs`

- Usar `git2::Repository::open(path)` para abrir o repositório
- Obter `git-common-dir` via `repo.commondir()` da crate `git2`
- Aplicar `std::fs::canonicalize` para resolver symlinks
- Implementar `is_same_repo(a, b)` comparando os `PathBuf` canonicalizados
- Listar worktrees ativas via `git worktree list --porcelain` (subprocess ou API `git2`)
- **Acceptance**: `RepoIdentity::from_path("/ProjetoA-feat")` retorna `repo_id == /ProjetoA/.git`

#### T11.2 — Atualizar `PrevalentState`

- Alterar `repos: HashMap<PathBuf, RepoIndex>` para `repos: HashMap<RepoId, RepoIndex>`
- Onde `RepoId = PathBuf` (alias de tipo, mas semanticamente o git-common-dir)
- Atualizar todos os sites de chamada que inserem/buscam no mapa
- **Acceptance**: `cargo build` sem erros; testes existentes de busca passam

#### T11.3 — Implementar `bare_commit.rs`

- Usar exclusivamente a API `git2-rs` (sem subprocessos shell)
- `ensure_branch()`: verifica `repo.find_branch("pks-knowledge", BranchType::Local)`; se `NotFound`, cria branch órfã com commit vazio inicial
- `write_file()`: segue o fluxo blob → treebuilder → commit descrito acima
- **Acceptance**: após `write_file`, `git show pks-knowledge:caminho/arquivo.md` retorna o conteúdo; `git status` mostra working tree limpa

#### T11.4 — Substituir lógica de commit

- Identificar em `journal.rs` (ou equivalente) onde ocorre o commit na `pks-knowledge`
- Substituir por `BareCommit::write_file`
- Remover código de checkout/stash que existia para contornar o problema anterior
- **Acceptance**: journaling funciona com worktrees secundárias ativas

#### T11.5 — Teste de identidade unificada

```rust
// pks/tests/test_repo_identity.rs
#[test]
fn two_worktrees_share_same_repo_id() {
    let tmp = tempdir().unwrap();
    // git init repo principal
    // git worktree add <tmp>/feat feature-branch
    let id_main = RepoIdentity::from_path(tmp.path()).unwrap();
    let id_feat = RepoIdentity::from_path(&tmp.path().join("feat")).unwrap();
    assert_eq!(id_main.repo_id, id_feat.repo_id);
}
```

#### T11.6 — Teste de bare commit

```rust
// pks/tests/test_bare_commit.rs
#[test]
fn bare_commit_does_not_dirty_working_tree() {
    let tmp = tempdir().unwrap();
    // git init + git worktree add feat
    let bc = BareCommit::new(tmp.path(), "pks-knowledge");
    bc.ensure_branch().unwrap();
    bc.write_file("index/test.md", b"conteudo", "test commit").unwrap();
    // Verificar que working tree está limpa
    let repo = git2::Repository::open(tmp.path()).unwrap();
    let statuses = repo.statuses(None).unwrap();
    assert!(statuses.is_empty(), "working tree deve estar limpa");
}
```

---

## Critérios de Aceite do M11

1. **RepoId estável**: `RepoIdentity::from_path(worktree_path).repo_id` é idêntico ao de qualquer outra worktree do mesmo repositório, incluindo a worktree principal.
2. **RepoId invariante a renomeação**: renomear a pasta da worktree não altera o `repo_id` derivado de `git-common-dir`.
3. **Bare Commit sem working tree sujo**: após `BareCommit::write_file(...)`, `git status` retorna working tree limpa. Verificável via `repo.statuses(None)?.is_empty()`.
4. **Sem `fatal: already checked out`**: gravar na `pks-knowledge` de uma worktree secundária não produz erro Git.
5. **`PrevalentState` unificado**: duas worktrees do mesmo repo compartilham o mesmo `RepoIndex` em memória no Daemon — sem duplicação de índice BM25.
6. **Testes T11.5 e T11.6 passam**: `cargo test test_repo_identity test_bare_commit` com saída verde.
7. **`cargo clippy -- -D warnings` limpo**: sem warnings nos módulos novos.

---

## Dependências Externas

| Crate | Versão mínima | Uso |
|-------|---------------|-----|
| `git2` | 0.18 | `Repository::open`, `repo.commondir()`, `treebuilder`, `repo.commit` — toda a lógica de RepoIdentity e BareCommit |
| `tempfile` | 3.8 | Criação de repositórios temporários nos testes T11.5 e T11.6 |

---

## Riscos e Mitigações

| Risco | Probabilidade | Mitigação |
|-------|---------------|-----------|
| **Symlink não resolvido** | Baixa | `std::fs::canonicalize` falha em symlinks quebrados → retornar `Err` com mensagem clara; o Daemon registra WARN e usa CWD como fallback temporário |
| **git-common-dir em repo sem Git** | Alta | `git2::Repository::open` falha → `RepoIdentity::from_path` retorna `Err(PksError::NotAGitRepo)`; nunca entra no `PrevalentState` |
| **Concorrência de indexação entre worktrees** | Média | duas worktrees disparam indexação simultânea com mesmo RepoId → `Arc<Mutex<RepoIndex>>` no Daemon serializa o acesso; debounce de 500ms por RepoId |
| **TreeBuilder race no BareCommit** | Baixa | duas escritas concorrentes na pks-knowledge criam divergência → BareCommit usa lock interno por repo_path antes de abrir o TreeBuilder |

---

## Impacto em Usuários Existentes

### Mudança no Formato do RepoId

O RepoId anterior era o `PathBuf` do CWD da sessão em que o PKS foi iniciado pela primeira vez. O novo RepoId é o `PathBuf` canônico do `git-common-dir`. Em repositórios sem worktrees extras, estes valores **geralmente coincidem** (ambos apontam para a pasta do projeto), mas **não são garantidamente iguais**:

- Em repos onde o `.git` é um arquivo (subtree, worktree), o valor muda
- Em repos acessados via symlink, `canonicalize` pode diferir do CWD

### Estratégia de Migração

O PKS não persiste o `RepoId` em disco entre sessões (o `PrevalentState` é reconstruído em memória a cada inicialização do Daemon). Portanto, **não há migração de dados persistidos** — a transição é automática na próxima inicialização.

A única área de impacto é o conteúdo da branch `pks-knowledge`, que não usa `RepoId` como chave (é indexada por caminho de arquivo dentro da tree Git). O conteúdo existente na `pks-knowledge` permanece válido e legível pelo novo `BareCommit`.

### Checklist para Equipes que Mantêm Forks

- [ ] Remover qualquer código que use `std::env::current_dir()` como chave de repositório
- [ ] Atualizar chamadas a `PrevalentState::get_repo(path)` para passar o path bruto — a resolução para `RepoId` ocorre internamente via `RepoIdentity::from_path`
- [ ] Verificar se há testes que assumem `repo_id == cwd` — ajustar para usar `repo_id == git_common_dir`

---

### 6. Observações Críticas (v2 Feedback)

- **Concorrência de Indexação:** Várias worktrees podem disparar indexação simultânea.
- **Dica:** Implementar **Debouncing/Serialização** de indexação por `RepoId` no Daemon Singleton.
