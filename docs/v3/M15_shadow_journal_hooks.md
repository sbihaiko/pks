# M15 — Shadow Journaling via Agent Hooks

| Campo | Valor |
|---|---|
| **Status** | PLANEJADO |
| **Depende de** | M11 (BareCommit), M14 (pks init) |
| **Complexidade** | Médio |
| **Arquivos principais** | `src/cli/mod.rs`, `src/cli/record_event.rs`, `src/cli/flush_session.rs`, `src/cli/submit_journal.rs` [NEW] |

---

## 1. Diagnóstico do M12 (por que falhou)

O M12 foi implementado com a arquitetura correta em isolamento, mas com dois erros de integração:

**Erro 1 — Ninguém chama `record_tool_event()`**
O MCP server registra `search_knowledge_vault` e `pks_execute`. Nenhum desses tools chama o hook. O IPC `RecordToolEvent` existe mas nunca é invocado pelo fluxo do agente.

**Erro 2 — `ipc/server.rs` quebrou o modelo de acumulação**
```rust
// ERRADO: cria nova instância por evento, perde acumulação
let mut hook = ShadowJournalHook::new(repo_path.clone(), session_id);
hook.record_tool_event(...);
hook.flush_to_vault(&bc);  // flush imediato = 1 commit por tool call
```

**Erro 3 — Acumulação em memória é incompatível com hooks**
Hooks são processos separados (`pks record-event` rodando como subprocess do Claude Code). Memória não é compartilhada entre invocações. Precisa de persistência intermediária.

---

## 2. Nova Arquitetura: Injeção Dupla (O Funil em 'Y')

Temos duas formas de coletar o journal baseadas na inteligência do assistente. Ambas convergem para o **BareCommit** no core Rust.

```text
[ Streaming Path - Claude ]            [ Batch Path - Antigravity ]
PostToolUse Hook (Events)              Fim do Workflow (/commit)
      │                                       │
pks record-event (JSONL)               Agente sintetiza resumo
      │                                       │
      ▼                                       ▼
pks flush-session                      pks submit-journal --agent
      │                                       │
      └───────────────> PKS Core <────────────┘
                        │
                (Gera Frontmatter)
                BareCommit -> pks-knowledge
                        │
                IPC PksCommand::Refresh -> Daemon
```

**Vantagens da arquitetura Dupla:**
- Atende IAs que trabalham ferramenta-a-ferramenta (Claude)
- Atende IAs de alto contexto guiadas por governança local/rules (Antigravity)
- Mantém o core do PKS imune aos detalhes de cada agente.

**Vantagens sobre a arquitetura v2:**
- Sem dependência de daemon ativo
- Sem IPC — processo direto
- Acumulação via arquivo sobrevive a crashes
- `session_id` vem do próprio hook payload (Claude Code fornece nativamente)

---

## 3. Novos Comandos CLI

### `pks record-event`

Lê JSON do stdin (payload PostToolUse do Claude Code), extrai campos relevantes e faz append em `~/.pks/sessions/{session_id}.jsonl`.

**Stdin esperado (payload Claude Code PostToolUse):**
```json
{
  "session_id": "abc123",
  "cwd": "/Users/user/project",
  "hook_event_name": "PostToolUse",
  "tool_name": "Edit",
  "tool_input": {
    "file_path": "src/main.rs",
    "old_string": "...",
    "new_string": "..."
  },
  "tool_response": {
    "success": true
  }
}
```

**Linha JSONL gerada** (`~/.pks/sessions/abc123.jsonl`):
```jsonl
{"timestamp":"2026-03-19T14:33:01Z","tool_name":"Edit","tool_input_summary":"src/main.rs","outcome":"success","file_paths":["src/main.rs"],"decision_note":null}
```

**Extração de `file_paths` por tipo de tool:**

| Tool | Campo em `tool_input` |
|---|---|
| Edit, Write, Read | `file_path` |
| MultiEdit | `file_path` (array de edits) |
| Bash | nenhum (outcome apenas) |
| Glob, Grep | nenhum (leitura, não grava) |

**Filtro de tools:** Só captura `Edit`, `Write`, `MultiEdit`, `Bash`. Ignora `Read`, `Glob`, `Grep`, `WebFetch` (sem valor de auditoria).

**Redação de secrets** aplicada ao `tool_input_summary` antes de gravar:
- `sk-[A-Za-z0-9_-]{20,}` → `[REDACTED_API_KEY]`
- `Bearer [A-Za-z0-9._-]{10,}` → `[REDACTED_BEARER]`
- `password=[^\s&]{4,}` → `[REDACTED_PASSWORD]`
- `token=[^\s&]{4,}` → `[REDACTED_TOKEN]`

**Comportamento em erro:** Silencioso — nunca bloqueia o Claude Code (exit 0 sempre).

---

### `pks flush-session <session_id>`

Lê `~/.pks/sessions/{session_id}.jsonl`, gera o markdown consolidado via `render_journal_md()`, commita na branch `pks-knowledge` via `BareCommit`, e apaga o arquivo de sessão.

**Stdin esperado (payload Claude Code Stop):**
```json
{
  "session_id": "abc123",
  "cwd": "/Users/user/project",
  "hook_event_name": "Stop",
  "stop_hook_active": false
}
```

**Guard contra loops:** Se `stop_hook_active == true`, encerra sem ação (exit 0) — evita loop infinito quando o Stop hook dispara outro Stop.

**Guard de sessão mínima:** Se o total de palavras nos summaries for menor que `PKS_JOURNAL_MIN_WORDS` (padrão: 10), descarta silenciosamente.

**Destino do arquivo:** `journals/{YYYY-MM-DD}_{session_id}.md` na branch `pks-knowledge` do repo em `cwd`.

**Comportamento em erro:** Graceful — loga warning, exit 0. Nunca bloqueia o encerramento da sessão.

---

### `pks submit-journal --agent <agente> --file <arquivo.md>`

Ingere resumos gerados dinamicamente no final de tarefas e aplica o commit no PKS. 
Lê um arquivo markdown específico, adiciona o cabeçalho base e os metadados e dispara para `commit_journal_entry()`.

**Fluxo esperado:**
Usado nativamente pelo script `.agent/scripts/checklist.py` ou workflows manuais quando o agente encerra seu ciclo funcional e emite resumos sob demanda.

---

## 4. Configuração de Hooks e Integração

### Claude Code (`.claude/settings.json`)

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write|MultiEdit|Bash",
        "hooks": [
          {
            "type": "command",
            "command": "pks record-event",
            "async": true
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "jq -r '\"\\(.session_id) \\(.cwd)\"' | read sid cwd; pks flush-session \"$sid\" \"$cwd\""
          }
        ]
      }
    ]
  }
}
```

**Alternativa mais robusta para o Stop hook** (script separado):

`.claude/hooks/pks-flush.sh`:
```bash
#!/bin/sh
INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id')
STOP_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active')
CWD=$(echo "$INPUT" | jq -r '.cwd')

# Guard: evita loop
if [ "$STOP_ACTIVE" = "true" ]; then exit 0; fi

pks flush-session "$SESSION_ID" "$CWD"
exit 0
```

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": ".claude/hooks/pks-flush.sh"
          }
        ]
      }
    ]
  }
}
```

### Antigravity

**Status: Aprovado novo Workflow Baseado em Checklist.**

O Antigravity **NÃO usará ganchos granulares (hooks)** de pre/post tool use para coletar logs reativos. O sistema segue uma mecânica orientada a *governanças* e *Phases*.

**Como irá funcionar:**
1. A regra primária no arquivo `GEMINI.md` dita que o agente finalize as *tasks* através da automação de checklist (`checklist.py`) ou por barra-comandos (`/commit` ou "son kontrolleri yap").
2. Durante este encerramento natural, instruiremos a LLM a gerar um *Shadow Journal* em Markdown em tempo real (já possuindo a janela de contexto de toda a sua sessão).
3. Isso dispara ou invoca diretamente:
   `pks submit-journal --agent "antigravity" --file journal_temporario.md`

Dessa maneira, a arquitetura do PKS e do Antigravity interagem fluidamente. O Agente mantém alta performance e velocidade sem overhead granular, usando sua janela global atenciosa para a documentação técnica.

---

## 5. Mudanças no Código Rust

### 5.1 Novos comandos no CLI

**`src/cli/mod.rs`** — adicionar variantes:
```rust
pub enum CliCommand {
    // ... existentes ...
    RecordEvent,                            // lê stdin
    FlushSession { session_id: String, cwd: PathBuf },
    SubmitJournal { agent: String, file: PathBuf },
}
```

**`src/cli/record_event.rs`** [NEW]:
```rust
pub fn run_record_event() -> i32 {
    // 1. Lê JSON do stdin
    // 2. Deserializa HookPayload { session_id, tool_name, tool_input, tool_response, cwd }
    // 3. Extrai file_paths baseado no tool_name
    // 4. Aplica redact_secrets() ao tool_input_summary
    // 5. Serializa JournalEntry como linha JSONL
    // 6. Append em ~/.pks/sessions/{session_id}.jsonl
    // 7. Exit 0 sempre (silencioso em erro)
}
```

**`src/cli/flush_session.rs`** [NEW]:
```rust
pub fn run_flush_session(session_id: &str, cwd: &Path) -> i32 {
    // 1. Lê ~/.pks/sessions/{session_id}.jsonl
    // 2. Deserializa Vec<JournalEntry>
    // 3. Guard: stop_hook_active check (via stdin ou flag)
    // 4. Guard: min_words check
    // 5. Instancia ShadowJournalHook com as entries
    // 6. Chama flush_to_vault() com BareCommit no cwd
    // 7. Deleta o arquivo JSONL
    // 8. Exit 0 sempre
}
```

**`src/cli/submit_journal.rs`** [NEW]:
```rust
pub fn run_submit_journal(agent: &str, file: &Path) -> i32 {
    // 1. Lê o conteúdo do arquivo markdown gerado pela IA
    // 2. Formata gerando obrigatoriamente data/hora em formato ISO 8601/RFC3339
    // 3. Obtém o PathBuf do cwd (ou workspace) e passa pro ShadowJournalHook
    // 4. Chama flush_to_vault() com BareCommit
    // 5. Comando IPC: IpcClient::send_command(&PksCommand::Refresh) para atualizar daemon
    // 6. Exit 0 em sucesso
}
```

### 5.2 Adaptação do `ShadowJournalHook`

Adicionar construtor alternativo que recebe `Vec<JournalEntry>` diretamente (para o flush-session que lê do JSONL):

```rust
impl ShadowJournalHook {
    pub fn from_entries(
        repo_path: PathBuf,
        session_id: String,
        started_at: DateTime<Utc>,
        entries: Vec<JournalEntry>,
    ) -> Self { ... }
}
```

### 5.3 Remover `RecordToolEvent` do IPC

**`src/ipc/mod.rs`** — remover variante `RecordToolEvent` do enum `PksCommand` **e** `PksResponse::EventRecorded`.
**`src/ipc/server.rs`** — remover o branch `PksCommand::RecordToolEvent` do `dispatch()`.

O IPC deixa de ser o canal de journaling. Hooks substituem completamente.

### 5.4 Novo tipo para payload do hook

**`src/hooks/hook_payload.rs`** [NEW]:
```rust
#[derive(Deserialize)]
pub struct PostToolUsePayload {
    pub session_id: String,
    pub cwd: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,  // estrutura varia por tool
    pub tool_response: ToolResponse,
}

#[derive(Deserialize)]
pub struct ToolResponse {
    pub success: bool,
}

#[derive(Deserialize)]
pub struct StopPayload {
    pub session_id: String,
    pub cwd: String,
    pub stop_hook_active: bool,
}
```

---

## 6. Armazenamento de Sessão

### Localização dos arquivos temporários

```
~/.pks/sessions/
├── abc123.jsonl          ← sessão ativa
├── def456.jsonl          ← outra sessão ativa
└── ...
```

**Formato JSONL** — uma linha por evento:
```jsonl
{"timestamp":"2026-03-19T14:33:01Z","tool_name":"Edit","tool_input_summary":"src/main.rs — old: fn foo, new: fn bar","outcome":"success","file_paths":["src/main.rs"],"decision_note":null}
{"timestamp":"2026-03-19T14:35:22Z","tool_name":"Bash","tool_input_summary":"cargo test -- retry","outcome":"success","file_paths":[],"decision_note":null}
```

**Limpeza:** O arquivo é deletado após flush bem-sucedido. Arquivos órfãos (sessões interrompidas sem Stop) podem ser limpos por `pks doctor` ou via TTL configurável.

---

## 7. Subtarefas

| ID | Tarefa | Arquivo | Depende de |
|---|---|---|---|
| T15.1 | Novo tipo `PostToolUsePayload` + `StopPayload` com deserialização | `src/hooks/hook_payload.rs` [NEW] | — |
| T15.2 | Implementar `run_record_event()` com extração de file_paths por tool | `src/cli/record_event.rs` [NEW] | T15.1 |
| T15.3 | Implementar `run_flush_session()` com leitura JSONL + guards | `src/cli/flush_session.rs` [NEW] | T15.1 |
| T15.4 | Implementar `run_submit_journal()` para batch ingestion | `src/cli/submit_journal.rs` [NEW] | — |
| T15.5 | Adicionar `ShadowJournalHook::from_entries()` | `src/hooks/shadow_journal.rs` | — |
| T15.6 | Registrar `record-event`, `flush-session`, `submit-journal` no parser do CLI | `src/cli/mod.rs` | T15.2, T15.3, T15.4 |
| T15.7 | Remover `RecordToolEvent` + `EventRecorded` do IPC | `src/ipc/mod.rs`, `src/ipc/server.rs` | — |
| T15.8 | Criar `.claude/settings.json` com configuração de hooks | `.claude/settings.json` [NEW] | T15.6 |
| T15.9 | Criar `.claude/hooks/pks-flush.sh` | `.claude/hooks/pks-flush.sh` [NEW] | T15.8 |
| T15.10 | Testes unitários para pipelines de Flush e Submit | `src/cli/*_tests.rs` | T15.2, T15.4 |
| T15.11 | Disparo de IPC Refresh: após commit, enviar notificação passiva para o daemon (detentor do lock) atualizar o Tantivy | `src/cli/flush_session.rs`, `src/cli/submit_journal.rs` | T15.3, T15.4 |

---

## 8. Riscos e Mitigações

| Risco | Probabilidade | Mitigação |
|---|---|---|
| **Stop hook não dispara** em crash ou kill -9 | Média | JSONL órfão em `~/.pks/sessions/`. Comando `pks doctor` lista e oferece flush manual. TTL configurável via `PKS_SESSION_TTL_HOURS` |
| **Journal não pesquisável** após commit | Alta (se omitido) | Disparo IPC (T15.11): após BareCommit, o CLI envia um `PksCommand::Refresh` local pro daemon importar imediatamente as novidades sem conflito de locks. |
| **Frontmatter sem hora** no Tantivy | Média | O Core Rust injeta data/hora obrigatoriamente em UTC (`RFC3339`) no frontmatter do journal gerado |
| **Loop infinito** no Stop hook | Baixa | Guard `stop_hook_active == true` → exit 0 imediato |
| **JSONL corrompido** por write parcial | Baixa | `flush-session` usa `serde_json::from_str` linha a linha — linhas inválidas são ignoradas com warning |
| **`cwd` errado** no flush (repo diferente) | Baixa | BareCommit busca git root a partir do `cwd` — erro claro se não for um repo git |
| **Múltiplas sessões simultâneas** no repo | Baixa | `session_id` único por sessão garante arquivos JSONL separados |

---

## 9. Critérios de Aceite

- [ ] `echo '<payload_post_tool_use>' | pks record-event` cria/atualiza `~/.pks/sessions/{session_id}.jsonl` sem I/O adicional
- [ ] `pks flush-session <id> <cwd>` gera `journals/YYYY-MM-DD_{id}.md` na branch `pks-knowledge` e apaga o JSONL
- [ ] `pks submit-journal --agent antigravity --file res.md` formata e aplica BareCommit via Workflow
- [ ] Após commit (flush ou submit), o CLI propaga um IPC Refresh passivo, não disparando panics de Tantivy Index Lock concorrentes
- [ ] Secrets no `tool_input` são redactados antes de gravar no JSONL
- [ ] `stop_hook_active: true` → flush-session encerra sem ação (exit 0)
- [ ] Qualquer erro silencioso nas rotinas CLI resulta em exit 0 e não trava as IAs
- [ ] `RecordToolEvent` e `EventRecorded` removidos do IPC sem breaking change observável
- [ ] `.claude/settings.json` presente no repo PKS e funcional
- [ ] Testes unitários e de integração validam as vias Batch e Streaming com sucesso
- [ ] `cargo test --workspace` verde após todas as mudanças
