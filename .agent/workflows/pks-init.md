# Workflow: /pks-init

**Nome:** pks-init
**Objetivo:** Inicializar o PKS (Prometheus Knowledge System) em um repositório Git com um único comando, em menos de 30 segundos.
**Versão PKS mínima:** v2 (M14)

---

## Pré-condições

- O diretório de trabalho deve estar dentro de um repositório Git (`git init` já executado).
- O binário `pks` deve estar disponível no PATH ou em `target/release/pks`.
- Rust toolchain instalada (se compilação for necessária).

---

## Passos

### 1. Executar `pks init`

```bash
pks init
```

Capturar o output completo. Se o output contiver `✓` em todas as linhas, a inicialização foi bem-sucedida.

### 2. Verificar output de sucesso

Output esperado:
```
✓ Git root detectado: <caminho>
✓ Config gerada: <caminho>/.pks/config.toml
✓ Branch criada: pks-knowledge
✓ Daemon registrado: <nome-projeto> (RepoId: <caminho>/.git)
PKS ativo. Buscar: pks search "<sua consulta>"
```

Se o output contiver `⚠ PKS já inicializado`, informar ao usuário e oferecer:
```bash
pks init --force  # sobrescreve config.toml sem apagar a branch
```

### 3. Ler `.pks/config.toml` gerado

Verificar os valores inferidos automaticamente:
- `name` = basename do diretório do projeto
- `git_common_dir` = caminho canônico do `.git` (invariante a worktrees)

Apresentar ao usuário os valores e confirmar se estão corretos.

### 4. Confirmar que o PKS está ativo

Executar uma busca de demonstração:
```bash
pks search "exemplo de busca"
```

Reportar ao usuário:
- Quantos arquivos foram indexados (se disponível via `pks refresh`)
- O `RepoId` registrado
- Status da branch `pks-knowledge`

### 5. Reportar resultado ao usuário (em PT-BR)

Formato de resposta:
```
PKS inicializado com sucesso em <nome-projeto>.

- Config: .pks/config.toml
- Branch: pks-knowledge (criada)
- RepoId: <git_common_dir>
- Modo de busca: BM25 puro (PKS_EMBEDDING_PROVIDER=none)

Para habilitar busca semântica vetorial (opcional):
  export PKS_EMBEDDING_PROVIDER=ollama
  ollama pull nomic-embed-text
```

---

## Tratamento de Erros

| Erro | Causa | Ação do LLM |
|------|-------|-------------|
| `não é um repositório Git` | Executado fora de um repo Git | Orientar a executar `git init` primeiro |
| `PKS já inicializado` | Segunda execução sem `--force` | Oferecer `pks init --force` |
| `Daemon offline` | PKS não responde via IPC/SSE | O PKS agora é operado via **stdio** no ambiente da IDE (confira `.mcp.json`). |
| Qualquer outro erro | Problema inesperado | Capturar mensagem de erro e apresentar em PT-BR com sugestão de solução |

Se o comando falhar, **não entrar em loop de retry**. Apresentar o erro ao usuário e aguardar instrução.

---

## Notas

- O `/pks-init` não modifica o `.gitignore` do usuário — usa `.git/info/exclude` (local, não commitado).
- A branch `pks-knowledge` é criada via git plumbing (sem checkout) — não afeta a working tree.
- Múltiplas worktrees do mesmo repo compartilham o mesmo `RepoId` — `pks init` é idempotente em worktrees secundárias.
