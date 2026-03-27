# STEERING: PKS v3 — Hardening, Compressão e Integração Obsidian

**Data:** Março de 2026
**Contexto:** O PKS completou M1–M17 (Fases A–E). O motor prevalente está funcional com busca híbrida BM25+Cosine, Git Journal, MCP federado e instalador one-click. A v3 foca em **corrigir dívidas estruturais**, **comprimir vetores para escala**, e **integrar o ecossistema Obsidian** como frontend de primeira classe.

**Referências:**
- [PRD](../PRD_prometheus_knowledge_system.md) — Visão completa do produto
- [STEERING_turboquant_pks.md](./STEERING_turboquant_pks.md) — Análise detalhada de compressão vetorial
- [PLAN_pks_core_fixes.md](../PLAN_pks_core_fixes.md) — Bugs estruturais identificados
- [Obsidian Agent Skills](https://github.com/kepano/obsidian-skills) — Skills reutilizáveis para agentes AI

---

## 1. Diagnóstico: O que a v2 entregou e o que ficou pendente

### Entregue (M1–M17)
- Motor Prevalente em RAM com Double-Buffered Pipeline (NYC-style)
- Busca híbrida BM25 (Tantivy) + Cosine SIMD + RRF (k=60)
- Daemon singleton via IPC (Unix Domain Socket)
- Git Journal Append com filtro Conventional Commits
- 5 ferramentas MCP: `search_knowledge_vault`, `list_knowledge_vaults`, `pks_execute`, `pks_add_decision`, `pks_add_feature`
- `pks init`, `pks doctor`, `pks refresh`, `pks status`, `pks validate`
- Degradação gradual de Ollama (5 estados)
- Snapshot bincode segmentado + Git LFS sync
- Instalador one-click (macOS/Linux/Windows)

### Dívida Estrutural Aberta
| # | Bug | Arquivo | Impacto |
|---|-----|---------|---------|
| B1 | ~~Snapshot isolation violation~~ | `state.rs:128-146` | ✅ **Corrigido em a173755/36393e2** — `chunks_for_repo_snapshot()` filtra por `repo_id` |
| B2 | **HashMap key bloat** — vetores indexados por `chunk.text` (500 bytes/key) em vez de `chunk_hash` (32 bytes) | `search/retriever.rs` | ~50MB de overhead para 100k chunks; bloqueia ganhos reais do TurboQuant |
| B3 | ~~Metadata loss in RRF para vector-only hits~~ | `search/hybrid.rs` | ✅ **Corrigido em c05c86e** |

---

## 2. Visão da v3: Três Pilares

```text
┌──────────────────────────────────────────────────────────────┐
│                        PKS v3                                │
│                                                              │
│  ┌─────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │  PILAR 1    │  │    PILAR 2      │  │    PILAR 3      │  │
│  │  Hardening  │  │  Compressão     │  │  Obsidian-      │  │
│  │  Estrutural │  │  & Escala       │  │  Native         │  │
│  │             │  │                 │  │                 │  │
│  │ • Fix B1/B2 │  │ • TurboQuant   │  │ • Agent Skills  │  │
│  │ • Testes    │  │ • HNSW futuro  │  │ • Wikilinks     │  │
│  │ • Snapshot  │  │ • HOT/COLD RAM │  │ • Graph-aware   │  │
│  │   v2 format │  │                 │  │ • Bases views   │  │
│  └─────────────┘  └─────────────────┘  └─────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

---

## 3. Pilar 1 — Hardening Estrutural (M18–M19)

### M18: Snapshot Isolation + Key Migration

**Objetivo:** Eliminar os dois bugs estruturais que impedem escala segura.

#### T18.1 — Fix `save_all_snapshots()` (B1)

**Problema:** O filtro em `state.rs:128-146` usa `filter(|_| true)`, despejando chunks de todos os repos em cada snapshot individual.

**Solução:**
```rust
// ANTES (bugado)
for (repo_id, _) in &self.repos {
    let chunks: Vec<_> = self.search_index.all_chunks()
        .filter(|_| true)  // ← BUG: não filtra por repo
        .collect();
    save_snapshot(repo_id, chunks);
}

// DEPOIS (correto)
for (repo_id, _) in &self.repos {
    let chunks: Vec<_> = self.search_index.all_chunks()
        .filter(|c| c.repo_id == *repo_id)
        .collect();
    save_snapshot(repo_id, chunks);
}
```

**Critérios de aceite:**
- [ ] Snapshot de repo A contém ZERO chunks de repo B (teste com 2 repos distintos)
- [ ] `pks validate` detecta inconsistência se snapshot contiver chunks estranhos
- [ ] Snapshot v2 format header (Magic Version bump) invalida snapshots v1 corrompidos

#### T18.2 — Migração de HashMap keys: `chunk.text` → `chunk_hash` (B2)

**Problema:** `HashMap<String, Vec<f32>>` em `retriever.rs` usa o texto integral do chunk como chave (~500 bytes). Para 100k chunks = ~50MB só de chaves.

**Solução:**
1. Alterar `vectors: HashMap<String, Vec<f32>>` para `vectors: HashMap<[u8; 32], Vec<f32>>`
2. Alterar `chunk_meta: HashMap<String, ChunkMeta>` para `chunk_meta: HashMap<[u8; 32], ChunkMeta>`
3. Atualizar todos os call sites em `retriever.rs`, `hybrid.rs`, `state.rs`
4. Manter backward compatibility temporária no desserializador de snapshots (lê formato antigo, salva no novo)

**Critérios de aceite:**
- [ ] Chaves do HashMap usam `[u8; 32]` (SHA-256 hash) — verificável via `std::mem::size_of`
- [ ] Busca híbrida retorna resultados idênticos antes/depois da migração (golden dataset)
- [ ] Snapshot v2 lê snapshots v1 com fallback; salva sempre em v2
- [ ] Redução mensurável de RSS: ≥40MB para 100k chunks

### M19: Robustez de Testes

**Objetivo:** Aumentar cobertura para permitir refatorações agressivas nos Pilares 2/3.

#### T19.1 — Testes de Snapshot Isolation
- Teste com 3 repos simultâneos: save → load → verify zero cross-contamination
- Property test: qualquer chunk carregado de `snapshot_A.bin` tem `repo_id == A`

#### T19.2 — Testes de Migração de Schema
- Snapshot v1 (keys textuais) → load com deserializer v2 → re-save → keys são `[u8; 32]`
- Snapshot corrompido → Magic Header mismatch → reindex completo (sem panic)

#### T19.3 — Golden Dataset Expandido
- Expandir de 50 para 100 notas cobrindo edge cases: notas vazias, headings sem corpo, unicode pesado, frontmatter YAML complexo
- Recall@5 ≥ 90% no golden dataset (BM25-only e híbrido)

---

## 4. Pilar 2 — Compressão & Escala (M20–M21)

### M20: TurboQuant — Compressão Vetorial

**Referência completa:** [STEERING_turboquant_pks.md](./STEERING_turboquant_pks.md)

**Pré-requisitos:** M18 (snapshot isolation + key migration) — OBRIGATÓRIO antes.

#### T20.1 — Implementação do Quantizador

1. **Randomized Hadamard Transform (RHT):**
   - Gerar seed determinística por vault (persistida no snapshot)
   - Implementar Fast Walsh-Hadamard Transform in-place (O(n log n))
   - Aplicar `Π = D·H` onde D é diagonal com sinais `{+1, -1}` derivados da seed

2. **Scalar Quantization (Max-Lloyd):**
   - Tabela pré-computada de thresholds para 3 bits/dim (8 níveis)
   - Quantização independente por coordenada após rotação
   - Resultado: 768 dims × 3 bits = 288 bytes/chunk (vs 3072 bytes em f32)

3. **QJL Residual Correction (1 bit):**
   - Projeção JL aleatória sobre resíduo `r = x - x̂`
   - Armazenar bit de sinal para correção de inner product
   - Total: 3.5 bits/dim = 336 bytes/chunk

**Layout em memória:**
```text
CompressedVector {
    quantized: [u8; 288],    // 768 dims × 3 bits, packed
    qjl_signs: [u8; 48],     // 384 bits de correção residual
    // Total: 336 bytes vs 3072 bytes (f32) = 9.1x compressão bruta
}
```

**Ganho real (com overhead do HashMap):**
| Componente | Antes (100k chunks) | Depois |
|------------|---------------------|--------|
| Vetores | 300 MB (f32) | ~33 MB (3.5 bits/dim) |
| Chaves HashMap | 50 MB (texto) → 3.2 MB (hash, M18) | 3.2 MB |
| ChunkMeta | ~30 MB | ~30 MB (inalterado) |
| **Total vetorial** | **~380 MB** | **~66 MB (~5.7x)** |

#### T20.2 — Busca com Vetores Quantizados

- Asymmetric Distance Computation (ADC): query em f32, database em quantizado
- Descomprimir on-the-fly durante cosine similarity (sem materializar f32 completo)
- Benchmark: latência p99 ≤ 15ms para 100k chunks (vs ~10ms atual em f32)

#### T20.3 — Validação de Qualidade

- Recall@5 vs baseline f32: ≥ 95% no golden dataset expandido (M19)
- A/B automático: `pks validate --compare-quant` roda queries do golden dataset em ambos os modos
- Se recall < 95%: fallback para 4 bits/dim (perde compressão, ganha qualidade)

**Critérios de aceite:**
- [ ] `CompressedVector` ocupa ≤ 340 bytes para 768 dims
- [ ] Recall@5 ≥ 95% vs f32 no golden dataset
- [ ] RSS total do Daemon com 100k chunks ≤ 100 MB
- [ ] Seed RHT persistida no snapshot; reconstrução determinística

### M21: HOT/COLD RAM (Preparação para Escala)

**Objetivo:** Gestão automática de memória para 10+ projetos simultâneos.

#### T21.1 — Política LRU Automática

- Repos acessados nas últimas 24h: HOT (índice completo em RAM)
- Repos inativos > 24h: WARM (BM25 em RAM, vetores em disco)
- Repos inativos > 7d: COLD (tudo em disco, load-on-demand)

#### T21.2 — Lazy Loading de Vetores

- Ao receber query para repo WARM: carrega vetores do snapshot sob demanda
- Promove repo para HOT automaticamente
- Timeout configurável: `PKS_HOT_TIMEOUT=24h`, `PKS_WARM_TIMEOUT=7d`

**Critérios de aceite:**
- [ ] Repos COLD não consomem RAM além de metadados (~100 bytes/repo)
- [ ] Primeira query em repo WARM→HOT: latência ≤ 500ms (load snapshot + search)
- [ ] Queries subsequentes em repo HOT: latência ≤ 15ms (normal)

---

## 5. Pilar 3 — Obsidian-Native (M22–M23)

### Contexto

O PKS gera `.md` automaticamente (Git Journal, tracker import, `pks_add_decision`, `pks_add_feature`) mas atualmente produz markdown genérico. O Obsidian oferece um ecossistema rico (wikilinks, Graph View, Bases, Canvas) que o PKS não aproveita. O projeto [Obsidian Agent Skills](https://github.com/kepano/obsidian-skills) padroniza a interação de agentes AI com vaults Obsidian.

### M22: Obsidian Flavored Markdown + Wikilinks

**Objetivo:** Toda nota gerada pelo PKS é nativamente navegável no Obsidian Graph View.

#### T22.1 — Wikilinks Automáticos

Quando o PKS gera uma nota (Git Journal, tracker import, `pks_add_*`), ele:

1. **Resolve referências cruzadas:** Se a nota menciona um conceito que já existe como nota no vault (ex: "retry pattern"), insere `[[Retry Pattern]]` automaticamente
2. **Backlinks de commit:** Entradas no Git Journal linkam para notas de feature/decision quando o commit message referencia um tracker_id: `- **14:30** - \`a1b2c3d\` - dev: feat: implementa [[Context_PAY-4421|PAY-4421]] retry`
3. **Índice BM25 como resolver:** O próprio Tantivy é usado para descobrir notas candidatas a wikilink (query rápida no título/heading do vault)

**Critérios de aceite:**
- [ ] Nota gerada por `pks_add_decision` contém wikilinks para notas existentes relacionadas
- [ ] Git Journal entry com tracker_id vira wikilink clicável no Obsidian
- [ ] Graph View do Obsidian mostra conexões entre decisões, features e logs diários
- [ ] Zero wikilinks quebrados (todas as referências apontam para notas existentes)

#### T22.2 — Properties YAML Padronizadas

Padronizar frontmatter YAML para que Obsidian Bases e plugins funcionem:

```yaml
---
type: decision | feature | log | tracker-import
created: 2026-03-27T14:00:00Z
source: git-journal | mcp-tool | tracker-import
related: ["[[ADR-007]]", "[[Context_PAY-4421]]"]
tags: [auth, retry, backend]
tracker_id: PAY-4421          # opcional, só para imports
commit_sha: a1b2c3d           # opcional, só para git journal
---
```

**Critérios de aceite:**
- [ ] Toda nota gerada pelo PKS tem frontmatter YAML válido com campo `type`
- [ ] Obsidian Properties view exibe os campos corretamente
- [ ] Campo `related` contém wikilinks funcionais

#### T22.3 — Integração com Obsidian Agent Skills

Instalar `obsidian-markdown` e `obsidian-cli` como skills disponíveis para o Claude Code ao trabalhar com vaults PKS:

1. `pks init` adiciona referência às skills no `.claude/settings.json` do projeto (ou equivalente)
2. O agente AI pode usar a skill `obsidian-markdown` para gerar notas com sintaxe Obsidian correta (callouts, embeds, etc.)
3. O agente AI pode usar `obsidian-cli` para interagir com o vault programaticamente

**Critérios de aceite:**
- [ ] `pks init` configura skills Obsidian automaticamente (opt-in via flag `--with-obsidian-skills`)
- [ ] Claude Code com skills ativas gera notas com callouts e embeds nativos
- [ ] `pks doctor` verifica se skills estão configuradas e sugere instalação

### M23: Obsidian Bases — Dashboards sobre o Vault

**Objetivo:** Views estruturadas sobre os dados do vault, sem UI custom.

#### T23.1 — Base de Decisões

Gerar automaticamente em `prometheus/05-decisions/_base.md`:
```
---
base: true
source: "05-decisions/"
columns: [type, created, tags, related]
sort: created desc
---
```

Dashboard filtrável de todas as ADRs, navegável no Obsidian.

#### T23.2 — Base de Sessões AI

Gerar em `prometheus/90-ai-memory/_base.md`:
- Agrupa logs diários por semana
- Mostra contagem de commits por dia
- Links para notas de decisão geradas na sessão

#### T23.3 — Base de Trackers

Gerar em `prometheus/02-features/_base.md`:
- Status do ticket (via `tracker_id` no frontmatter)
- Última sincronização (`synced_at`)
- Filtro por tracker (Jira, Notion, Linear)

**Critérios de aceite:**
- [ ] `pks init` gera os 3 arquivos Base automaticamente
- [ ] Bases abrem corretamente no Obsidian com colunas e filtros funcionais
- [ ] Novas notas geradas pelo PKS aparecem automaticamente nas Bases relevantes

---

## 6. Ordem de Execução e Dependências

```text
M18 (Hardening)──────────► M20 (TurboQuant)──────► M21 (HOT/COLD)
  │                           │
  └──► M19 (Testes)───────────┘
                                   (independente)
M22 (Obsidian MD)─────────► M23 (Bases)
```

**Paralelismo possível:**
- Pilar 1 (M18/M19) e Pilar 3 (M22) podem rodar em paralelo — não compartilham código
- M20 depende estritamente de M18 (keys precisam ser hash antes de comprimir vetores)
- M21 depende de M20 (lazy loading faz mais sentido com vetores comprimidos)
- M23 depende de M22 (Bases usam o frontmatter YAML padronizado)

### Sugestão de Fases

| Fase | Milestones | Foco |
|------|-----------|------|
| **v3.0** | M18 + M19 + M22 | Hardening + Obsidian MD (paralelo) |
| **v3.1** | M20 + M23 | TurboQuant + Bases |
| **v3.2** | M21 | HOT/COLD RAM |

---

## 7. Decisões Pendentes

| # | Questão | Opções | Recomendação |
|---|---------|--------|-------------|
| D15 | Wikilink resolution: tempo de geração vs lazy? | Resolver no momento de gerar a nota vs resolver em batch periódico | **Tempo de geração** — o BM25 é sub-ms, custo negligível |
| D16 | TurboQuant bits/dim default | 2.5 vs 3.0 vs 3.5 | **3.5 bits** — priorizar qualidade; comprimir mais só se RAM pressionar |
| D17 | Obsidian Skills: bundled vs opt-in? | Instalar automaticamente vs flag `--with-obsidian-skills` | **Opt-in** — nem todo usuário usa Obsidian como frontend |
| D18 | Snapshot format v2: breaking change ou backward compat? | Migração transparente vs force reindex | **Migração transparente** — lê v1, salva v2 |
| D19 | HNSW: implementar em v3 ou postergar? | v3 vs v4 | **Postergar para v4** — O(N) scan com TurboQuant aguenta 100k+ chunks em <15ms |

---

## 8. Riscos

| Risco | Probabilidade | Impacto | Mitigação |
|-------|--------------|---------|-----------|
| TurboQuant degrada qualidade abaixo de 95% recall | Média | Alto | Fallback configurável para 4 bits/dim; A/B automático no golden dataset |
| Obsidian Bases spec muda (feature recente) | Baixa | Médio | Gerar Bases simples; manter gerador desacoplado |
| Key migration corrompe snapshots existentes | Baixa | Alto | Migração transparente com fallback; backup automático antes de converter |
| Obsidian Skills não amadurecem | Média | Baixo | Skills são opt-in; funcionalidade core (wikilinks, YAML) não depende delas |

---

## 9. Métricas de Sucesso da v3

| Métrica | Alvo |
|---------|------|
| RSS do Daemon (100k chunks, 3 repos) | ≤ 100 MB (vs ~380 MB atual) |
| Recall@5 híbrido (golden dataset) | ≥ 95% |
| Zero cross-contamination em snapshots | 100% dos testes passando |
| Notas geradas com wikilinks válidos | 100% |
| Graph View conectividade | ≥ 3 edges/nota (média) |
| Tempo de `pks init` com Obsidian setup | ≤ 45s |
