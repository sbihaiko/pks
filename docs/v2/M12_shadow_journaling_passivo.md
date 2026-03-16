# M12 — Shadow Journaling Passivo

| Campo         | Valor                         |
|---------------|-------------------------------|
| **Status**    | PENDENTE                      |
| **Depende de**| M11 (BareCommit via git2-rs)  |
| **Arquivo**   | `pks/src/hooks/shadow_journal.rs` |
| **Branch de saída** | `pks-knowledge`         |

---

## 1. Objetivo

Cada sessão de pair-programming com uma IA gera decisões, refatorações e contexto técnico valioso. Quando o terminal fecha, todo esse contexto desaparece.

O M12 resolve isso de forma **completamente passiva**: sem que o usuário ou o agente precisem executar nenhum comando adicional, cada sessão LLM gera automaticamente um arquivo de journal estruturado na branch `pks-knowledge`. O PKS passa a ser um sistema que **acumula memória técnica** de todas as sessões anteriores, tornando buscas futuras muito mais ricas.

---

## 2. Como Funciona

```
Sessão LLM ativa
      │
      ▼ (cada chamada de ferramenta)
PostToolUse hook (Claude Code / agente MCP)
      │
      ▼ IPC (Unix Socket / Named Pipe)
pks record-tool-event --session-id <id> --tool <name> --outcome <ok|err>
      │
      ▼
ShadowJournalHook::record_tool_event()
  └─► acumula Vec<JournalEntry> em memória (RAM, zero I/O)
      │
      ▼ (fim da sessão)
ShadowJournalHook::flush_to_vault()
  └─► render_journal_md() → String Markdown
  └─► BareCommit::write_file() → refs/heads/pks-knowledge
      │
      ▼
journals/YYYY-MM-DD_session-{id}.md
```

O hook é **fire-and-forget do ponto de vista do agente**: o proxy MCP envia o evento via IPC com timeout de 50ms e continua independentemente de resposta. O Daemon processa em background sem bloquear a sessão.

---

## 3. Estruturas de Dados

Arquivo: `pks/src/hooks/shadow_journal.rs`

```rust
pub struct ShadowJournalHook {
    repo_path: PathBuf,
    session_id: String,
    started_at: DateTime<Utc>,
    entries: Vec<JournalEntry>,
}

#[derive(Serialize)]
pub struct JournalEntry {
    pub timestamp: DateTime<Utc>,
    pub tool_name: String,          // "Edit", "Bash", "Write", etc.
    pub tool_input_summary: String, // truncado em 200 chars
    pub outcome: String,            // "success" | "error" | "denied"
    pub file_paths: Vec<String>,    // caminhos relativos ao repo root
    pub decision_note: Option<String>, // texto livre opcional do agente
}
```

Métodos principais:

```rust
impl ShadowJournalHook {
    pub fn new(repo_path: PathBuf) -> Self;

    // Chamado a cada PostToolUse — zero I/O, apenas push no Vec
    pub fn record_tool_event(&mut self, event: ToolEvent);

    // Chamado ao fim da sessão — grava via BareCommit
    pub fn flush_to_vault(&self, bare_commit: &BareCommit) -> Result<()>;

    // Gera o conteúdo Markdown do journal
    fn render_journal_md(&self) -> String;
}
```

---

## 4. Formato do Journal

Caminho no branch `pks-knowledge`: `journals/YYYY-MM-DD_session-{id}.md`

```markdown
# Session Journal: 2026-03-15 — session-abc123

**Projeto:** payment-api
**Início:** 2026-03-15T14:32:00Z
**Duração:** 45min
**Ferramentas usadas:** Edit (12), Bash (8), Write (3)

## Decisões

- Movida lógica de retry para struct `RetryPolicy` (Edit: `src/retry.rs`)
- Adicionado backoff exponencial com jitter (Bash: `cargo test`)
- Novos testes de integração criados (Write: `tests/retry_test.rs`)

## Arquivos Modificados

| Arquivo                  | Operação | Resumo                        |
|--------------------------|----------|-------------------------------|
| `src/retry.rs`           | Edit     | +120 linhas — RetryPolicy     |
| `tests/retry_test.rs`    | Write    | +85 linhas — testes e2e       |
| `Cargo.toml`             | Edit     | adicionada dep `rand`         |

## Eventos Detalhados

| Timestamp            | Ferramenta | Outcome  | Paths                     |
|----------------------|------------|----------|---------------------------|
| 2026-03-15T14:33:01Z | Edit       | success  | src/retry.rs              |
| 2026-03-15T14:35:22Z | Bash       | success  | —                         |
| 2026-03-15T14:40:10Z | Write      | success  | tests/retry_test.rs       |
```

---

## 5. Configuração

O Shadow Journaling é controlado por variáveis de ambiente. O sistema funciona com zero configuração via defaults seguros.

| Variável                  | Padrão  | Descrição                                               |
|---------------------------|---------|---------------------------------------------------------|
| `PKS_SHADOW_JOURNAL`      | `true`  | Habilita/desabilita o journaling passivo                |
| `PKS_JOURNAL_MIN_WORDS`   | `10`    | Sessões com menos palavras no resumo são descartadas    |
| `PKS_JOURNAL_MAX_ENTRIES` | `500`   | Máximo de entradas por sessão (proteção contra floods)  |
| `PKS_JOURNAL_TRUNCATE`    | `200`   | Limite de chars para `tool_input_summary`               |

Configuração via `.pks.yaml` (opcional, sobrescreve env vars):

```yaml
shadow_journal:
  enabled: true
  min_words: 10
  max_entries: 500
  truncate_chars: 200
```

Se nem `.pks.yaml` nem variáveis de ambiente estiverem presentes, os defaults acima são aplicados em hardcode no binário Rust.

---

## 6. Subtarefas

| ID    | Tarefa                                                                 | Arquivo                                  | Depende de |
|-------|------------------------------------------------------------------------|------------------------------------------|------------|
| T12.1 | Implementar struct `JournalEntry` + serialização `serde::Serialize`    | `pks/src/hooks/shadow_journal.rs`        | —          |
| T12.2 | Implementar `ShadowJournalHook::record_tool_event()`                   | `pks/src/hooks/shadow_journal.rs`        | T12.1      |
| T12.3 | Implementar `render_journal_md()` — gerar Markdown estruturado         | `pks/src/hooks/shadow_journal.rs`        | T12.2      |
| T12.4 | Implementar `flush_to_vault()` — gravar via `BareCommit`               | `pks/src/hooks/shadow_journal.rs`        | T12.3, M11 |
| T12.5 | Adicionar comando IPC `RecordToolEvent` ao enum `PksCommand`           | `pks/src/ipc/mod.rs`                     | M10        |
| T12.6 | Teste de integração: simular 5 eventos, flush, verificar arquivo no branch `pks-knowledge` | `pks/tests/shadow_journal_e2e.rs` | T12.4, T12.5 |

### Detalhamento das Subtarefas

**T12.1 — JournalEntry struct**
Adicionar `#[derive(Serialize, Debug, Clone)]` à struct. Incluir validação de truncamento do campo `tool_input_summary` no construtor.

**T12.2 — record_tool_event()**
Recebe um `ToolEvent` (nome da ferramenta, outcome, paths), aplica truncamento e faz `self.entries.push(entry)`. Verificar `max_entries` para evitar crescimento ilimitado.

**T12.3 — render_journal_md()**
Agregar eventos por ferramenta para a seção "Ferramentas usadas". Extrair `decision_note` presentes para a seção "Decisões". Formatar tabela de arquivos modificados sem duplicatas.

**T12.4 — flush_to_vault()**
Chamar `render_journal_md()` para obter o conteúdo. Construir o path `journals/YYYY-MM-DD_session-{id}.md`. Delegar escrita para `BareCommit::write_file()`. Retornar `Ok(())` ou logar erro sem panic (graceful failure).

**T12.5 — IPC RecordToolEvent**
Adicionar variante ao enum `PksCommand`:
```rust
RecordToolEvent {
    session_id: String,
    tool_name: String,
    outcome: String,
    file_paths: Vec<String>,
    decision_note: Option<String>,
},
```

**T12.6 — Teste e2e**
```rust
// pks/tests/shadow_journal_e2e.rs
// 1. Criar repo Git temporário com branch pks-knowledge
// 2. Instanciar ShadowJournalHook
// 3. Chamar record_tool_event() 5 vezes com ferramentas distintas
// 4. Chamar flush_to_vault()
// 5. Ler blob do branch pks-knowledge via git2
// 6. Verificar que o arquivo journals/YYYY-MM-DD_session-*.md existe
// 7. Verificar seções obrigatórias: "Decisões", "Arquivos Modificados"
```

---

## Dependências Externas

| Crate | Versão mínima | Uso |
|-------|---------------|-----|
| `serde` | 1.0 | `Serialize`/`Deserialize` para `JournalEntry` e `ToolEvent` |
| `chrono` | 0.4 | `DateTime<Utc>` para timestamps de sessão e entradas |
| `uuid` | 1.6 | Geração de `session_id` único por sessão |
| `git2` | 0.18 | Via `BareCommit` (M11) — escrita na branch `pks-knowledge` |

---

## Riscos e Mitigações

| Risco | Probabilidade | Mitigação |
|-------|---------------|-----------|
| **Secrets no journal**: tool_input_summary pode capturar fragmentos de tokens ou senhas | Alta | Regex de padrões comuns (`sk-`, `Bearer `, `password=`, UUID-like strings) aplicada antes de `flush_to_vault`; campos que disparam o regex são substituídos por `[REDACTED]` |
| **Daemon offline no flush**: IPC timeout ao tentar gravar no fim da sessão | Média | `flush_to_vault` com timeout de 5s; em falha, grava journal em `~/.pks/pending_journals/` para re-tentativa na próxima inicialização do Daemon |
| **Sessão muito longa (> PKS_JOURNAL_MAX_ENTRIES)**: Vec cresce indefinidamente | Baixa | Guard em `record_tool_event`: se `entries.len() >= max_entries`, descarta novas entradas e incrementa contador `dropped_events` para incluir no journal |
| **Colisão de session_id**: dois processos geram o mesmo ID | Muito Baixa | UUID v4 tem colisão negligível; prefixo de PID como fallback: `{pid}-{uuid4}` |

---

## 7. Critérios de Aceite do M12

- [ ] `ShadowJournalHook::record_tool_event()` não realiza nenhuma operação de I/O — apenas acumula em memória.
- [ ] `flush_to_vault()` grava exatamente um arquivo no caminho `journals/YYYY-MM-DD_session-{id}.md` na branch `pks-knowledge` via `BareCommit`.
- [ ] Arquivo de journal gerado contém as seções: cabeçalho, Decisões, Arquivos Modificados, Eventos Detalhados.
- [ ] Com `PKS_SHADOW_JOURNAL=false`, nenhum arquivo de journal é gravado — `flush_to_vault()` retorna `Ok(())` sem ação.
- [ ] Sessão com menos de `PKS_JOURNAL_MIN_WORDS` palavras no resumo é descartada silenciosamente.
- [ ] Se o Daemon estiver offline, o hook IPC não bloqueia a sessão (timeout de 50ms, engole erro).
- [ ] Teste T12.6 passa em CI sem dependências externas (usa repo temporário em `/tmp`).
- [ ] Nenhum dado gravado no journal contém secrets detectáveis (tokens, senhas) — validação básica de padrão regex antes de `flush_to_vault()`.

---

## 8. Privacidade e Controle

### O que NÃO é gravado

- Conteúdo completo de arquivos (apenas paths e resumo truncado).
- Outputs de comandos Bash (apenas nome do comando e outcome).
- Variáveis de ambiente, tokens ou credenciais.
- Entradas do usuário fora do contexto de chamadas de ferramentas.

### Como desabilitar

```bash
# Desabilitar globalmente
export PKS_SHADOW_JOURNAL=false

# Ou via .pks.yaml no repositório
printf 'shadow_journal:\n  enabled: false\n' >> .pks.yaml
```

### Como deletar entradas existentes

Os arquivos de journal vivem na branch `pks-knowledge`. Para remover entradas, use
`BareCommit` diretamente ou via CLI do PKS (futuramente `pks journal remove <date>`).
O PKS **nunca deleta** arquivos de journal de forma autônoma.

```bash
# Ver journals existentes
git log pks-knowledge -- journals/

# Para remover manualmente: use git plumbing (sem checkout) ou aguarde o comando
# `pks journal remove` previsto em versões futuras.
```

---

### 9. Observações Críticas (v2 Feedback)

- **Segurança de Dados:** Regex para secrets é falho. 
- **Aviso:** A branch `pks-knowledge` deve ser tratada como sensível; considere heurísticas de busca de secrets antes do flush.

