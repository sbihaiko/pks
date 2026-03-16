# M13 — Ollama Opcional + pks_execute (Context-Mode Integration)

| Campo         | Valor                                      |
|---------------|--------------------------------------------|
| **Status**    | PENDENTE                                   |
| **Depende de**| M10 (IPC), M11 (BareCommit)                |
| **Prioridade**| Alta                                       |

---

## Visão Geral

M13 torna o PKS completamente funcional sem Ollama instalado e adiciona a ferramenta
MCP `pks_execute`, que executa código em sandbox e devolve apenas um resumo indexado
ao LLM — prevenindo o flooding do contexto.

---

## Parte A: Ollama Opcional

### Problema

O workflow `.agent/workflows/pks-install.md` instala Ollama obrigatoriamente e faz pull
do modelo `nomic-embed-text`. Isso bloqueia usuários que:

- Não têm GPU/RAM suficiente para rodar Ollama localmente.
- Querem usar o PKS em ambientes de CI/CD ou servidores headless.
- Precisam de busca rápida (BM25) sem latência de embeddings.

O mecanismo BM25 do PKS já funciona de forma independente. O erro só ocorre porque o
pipeline de embeddings tenta contatar `http://127.0.0.1:11434` incondicionalmente na
inicialização do daemon.

### Solução: `PKS_EMBEDDING_PROVIDER`

Introduzir a variável de ambiente `PKS_EMBEDDING_PROVIDER` lida pelo daemon PKS na
inicialização. O valor padrão é `none`, tornando BM25 o modo default — sem Ollama.

#### Tabela de Modos

| `PKS_EMBEDDING_PROVIDER` | Busca BM25 | Busca Vetorial | Ollama necessário | Observação                          |
|--------------------------|------------|----------------|-------------------|-------------------------------------|
| `none` (padrão)          | Sim        | Não            | Não               | Modo leve; funciona em qualquer env |
| `ollama`                 | Sim        | Sim            | Sim               | Híbrido BM25 + vetor                |
| `mlx`                    | Sim        | Sim            | Não               | Apple MLX — implementação futura    |

#### Detecção na Inicialização do Daemon

```rust
// pks/src/daemon/startup.rs
fn resolve_embedding_provider() -> EmbeddingProvider {
    match std::env::var("PKS_EMBEDDING_PROVIDER")
        .unwrap_or_else(|_| "none".to_string())
        .to_lowercase()
        .as_str()
    {
        "ollama" => EmbeddingProvider::Ollama,
        "mlx"   => EmbeddingProvider::Mlx,
        _       => EmbeddingProvider::None,
    }
}
```

#### Impacto no Armazenamento de Chunks

O campo `embedding` no struct de chunk já deve aceitar `None` quando o provedor é
`none`, permitindo indexação e busca BM25 sem vetor associado:

```rust
// pks/src/storage/chunk.rs
pub struct Chunk {
    pub id:        Uuid,
    pub vault_id:  RepoId,
    pub text:      String,
    pub metadata:  ChunkMetadata,
    pub embedding: Option<Vec<f32>>,  // None quando provider = none (graceful degradation)
}
```

Chunks com `embedding: None` são servidos exclusivamente via BM25. Se o provedor for
alterado para `ollama` posteriormente, o daemon reindexará os chunks pendentes em
background sem exigir reindexação manual.

### Mudanças no pks-install.md

- Remover a instalação obrigatória do Ollama e o `ollama pull nomic-embed-text`.
- Adicionar bloco condicional:

```
## Passo opcional: Busca Semântica com Ollama

Se quiser busca vetorial além de BM25, instale o Ollama separadamente:

  brew install --cask ollama          # macOS
  ollama pull nomic-embed-text
  export PKS_EMBEDDING_PROVIDER=ollama

Sem esse passo, o PKS opera em modo BM25 puro (padrão).
```

---

## Parte B: pks_execute — Context-Mode Integration

### Problema

Ferramentas como `Bash` e `Read` devolvem o output bruto ao contexto do LLM. Em
comandos de alto volume (ex: `cargo test`, `npm run build`, logs de servidor), isso
pode consumir dezenas de milhares de tokens — bloqueando raciocínio e aumentando custo.

Exemplos de outputs problemáticos:

| Comando              | Linhas típicas | Tokens estimados |
|----------------------|----------------|------------------|
| `cargo test`         | 400–2000       | 8k–40k           |
| `npm run build`      | 200–800        | 4k–16k           |
| `git log --stat`     | 100–500        | 2k–10k           |
| `grep -r ... src/`   | 50–300         | 1k–6k            |

### Solução: pks_execute

`pks_execute` é uma ferramenta MCP que:

1. Recebe código + linguagem + intenção de busca.
2. Executa o código em um subprocess isolado (sandbox).
3. Indexa o output completo no PKS.
4. Retorna apenas um **resumo** — as N seções mais relevantes para a `intent`.

#### Diagrama de Fluxo

```
LLM
 │
 │  pks_execute({ language: "shell", code: "cargo test", intent: "failing tests" })
 ▼
MCP Server (PKS daemon)
 │
 ├─► Subprocess / Sandbox
 │       └─ executa código com timeout
 │       └─ captura stdout + stderr (output bruto — NUNCA enviado ao LLM)
 │
 ├─► Indexador PKS
 │       └─ divide output em chunks
 │       └─ indexa via BM25 (+ vetor se PKS_EMBEDDING_PROVIDER=ollama)
 │
 └─► Busca por `intent`
         └─ retorna top-N chunks relevantes
         └─ monta ExecuteResponse com summary + metadados
 │
 ▼
LLM recebe apenas o resumo (dezenas de linhas, não milhares)
```

#### Structs Rust

```rust
// pks/src/mcp/tools/pks_execute.rs

pub struct PksExecuteTool;

pub struct ExecuteParams {
    pub language:   String,           // "shell" | "python" | "javascript" | ...
    pub code:       String,           // comando ou script a executar
    pub intent:     Option<String>,   // o que buscar no output (ex: "failing tests")
    pub timeout_ms: Option<u64>,      // padrão: 30_000 ms
}

pub struct ExecuteResponse {
    pub summary:          String,       // top-N seções relevantes ao intent
    pub total_lines:      usize,        // total de linhas no output bruto (transparência)
    pub indexed:          bool,         // true se output foi indexado com sucesso
    pub searchable_terms: Vec<String>,  // vocabulário para buscas de follow-up
}

impl PksExecuteTool {
    /// Ferramenta MCP: pks_execute
    /// Executa código em sandbox, indexa output e retorna apenas resumo.
    pub fn execute(params: ExecuteParams) -> Result<ExecuteResponse> {
        let raw_output = run_in_sandbox(&params.language, &params.code, params.timeout_ms)?;
        let total_lines = raw_output.lines().count();
        let chunks = chunk_output(&raw_output);
        let index_id = index_chunks(chunks)?;
        let intent = params.intent.unwrap_or_else(|| "errors warnings summary".to_string());
        let top_chunks = search_index(index_id, &intent, TOP_N)?;
        let summary = render_summary(&top_chunks);
        let searchable_terms = extract_vocabulary(&top_chunks);
        Ok(ExecuteResponse {
            summary,
            total_lines,
            indexed: true,
            searchable_terms,
        })
    }
}
```

#### Comparação: Bash vs pks_execute

| Aspecto                  | `Bash` (raw)                     | `pks_execute` (summary)           |
|--------------------------|----------------------------------|-----------------------------------|
| Output ao LLM            | Bruto (todas as linhas)          | Resumo (top-N relevantes)         |
| Consumo de tokens        | Alto (8k–40k por chamada)        | Baixo (<500 tokens por chamada)   |
| Busca por intent         | Não                              | Sim (BM25 ou vetorial)            |
| Indexação para follow-up | Não                              | Sim (searchable_terms retornados) |
| Sandbox / timeout        | Não                              | Sim (timeout_ms configurável)     |
| Uso recomendado          | Comandos curtos (<20 linhas)     | Comandos de alto volume           |

---

## Subtarefas

| ID    | Descrição                                                                 | Arquivo alvo                                   | Estimativa |
|-------|---------------------------------------------------------------------------|------------------------------------------------|------------|
| T13.1 | Adicionar leitura de `PKS_EMBEDDING_PROVIDER` na inicialização do daemon  | `pks/src/daemon/startup.rs`                    | 2h         |
| T13.2 | Tornar pipeline de embeddings condicional — skip se `provider = none`     | `pks/src/embeddings/pipeline.rs`               | 3h         |
| T13.3 | Atualizar `pks-install.md`: marcar Ollama como passo opcional             | `.agent/workflows/pks-install.md`              | 1h         |
| T13.4 | Implementar `ExecuteParams` + `ExecuteResponse` (structs + validação)     | `pks/src/mcp/tools/pks_execute.rs`             | 2h         |
| T13.5 | Implementar `PksExecuteTool::execute()` com subprocess sandbox            | `pks/src/mcp/tools/pks_execute.rs`             | 4h         |
| T13.6 | Registrar `pks_execute` como ferramenta MCP no servidor                   | `pks/src/mcp/server.rs`                        | 1h         |
| T13.7 | Teste de integração: executar shell via `pks_execute`, verificar summary  | `pks/tests/pks_execute_integration_test.rs`    | 3h         |

**Total estimado: 16h**

---

## Critérios de Aceite do M13

1. **BM25 sem Ollama**: `PKS_EMBEDDING_PROVIDER=none` (ou variável ausente) inicia o daemon
   sem erros, indexa documentos e retorna resultados de busca BM25 funcionais.

2. **Degradação graciosa**: Chunks indexados com `embedding: None` são servidos via BM25
   sem pânico ou erro. Log INFO indica modo ativo na inicialização.

3. **Ollama condicional**: Apenas com `PKS_EMBEDDING_PROVIDER=ollama` o daemon tenta
   contatar `http://127.0.0.1:11434`. Sem essa variável, nenhuma conexão é tentada.

4. **pks_execute funcional**: A ferramenta MCP `pks_execute` aceita `language`, `code` e
   `intent`, executa em sandbox com timeout, e retorna `ExecuteResponse` com `summary`,
   `total_lines`, `indexed: true` e `searchable_terms`.

5. **Prevenção de flooding**: Para outputs com >50 linhas, `pks_execute` retorna um
   `summary` com no máximo 30 linhas (top-N chunks), independente do tamanho do output bruto.

6. **pks-install.md atualizado**: Ollama não aparece mais em passos obrigatórios. Bloco
   condicional claro documenta como ativar busca semântica opcionalmente.

7. **Teste de integração passa**: `T13.7` executa `echo "linha\n"` repetido 500x via
   `pks_execute`, verifica que `total_lines >= 500`, `indexed == true` e `summary.len() < 2000`.

---

## Métricas de Sucesso

| Métrica                                         | Meta        | Método de medição                               |
|-------------------------------------------------|-------------|-------------------------------------------------|
| Redução de tokens no contexto (outputs grandes) | > 60%       | Comparar tokens de `Bash` vs `pks_execute` com `cargo test` |
| Tempo de resposta `pks_execute` (shell, 1k linhas) | < 2s     | Benchmark no teste T13.7                        |
| Taxa de erro com `PKS_EMBEDDING_PROVIDER=none`  | 0%          | Suite de testes existente sem Ollama rodando    |
| Cobertura de teste de T13.7                     | 100% happy path + 2 edge cases | CI                         |

---

## Notas de Implementação

- O sandbox de `pks_execute` deve usar `std::process::Command` com `timeout` explícito.
  Em Unix, usar `kill(-pid, SIGKILL)` no processo filho ao expirar o timeout.
- `TOP_N` (número de chunks no summary) deve ser configurável via `PKS_EXECUTE_TOP_N`
  (padrão: `5`).
- O índice criado por `pks_execute` é efêmero (in-memory ou TTL de 1h) para não poluir
  os vaults permanentes.
- `searchable_terms` devem ser extraídos com TF-IDF simples sobre os chunks retornados,
  permitindo follow-up com `pks search <term>` sem reindexação.

---

### 10. Observações Críticas (v2 Feedback)

- **Escalabilidade de Memória:** O Daemon pode acumular muitos índices. 
- **Dica:** Adicionar política de **Eviction (LRU)** para descarregar `tantivy::Index` inativos.
- **UX de Ferramentas:** Diferenciar claramente `Bash` vs `pks_execute` na descrição do MCP para o LLM.

