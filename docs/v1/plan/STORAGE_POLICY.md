# Storage Policy — Prometheus Knowledge System

## Principio Fundamental

> **prometheus/ guarda contexto e raciocinio, nao a informacao em si.**

O diretorio `prometheus/` de cada vault armazena apenas artefatos que capturam
**por que** decisoes foram tomadas, **como** problemas foram resolvidos e **qual**
raciocinio guiou o trabalho. Dados brutos, binarios e conteudo efemero ficam de fora.

## Tipos de Conteudo

| Tipo | Tag detectada | Decisao | Justificativa |
|------|--------------|---------|---------------|
| TrackerImport | `tracker`, `import` | STORE | Contexto de sessao de trabalho |
| AiSummary | `ai-summary`, `session-summary` | STORE | Raciocinio sintetizado por IA |
| Adr | `adr`, `decision` | STORE | Registro arquitetural de decisoes |
| ManualNote | `note`, `manual` | STORE | Anotacoes contextuais do usuario |
| Runbook | `runbook`, `playbook` | STORE | Procedimentos operacionais |
| Unknown | (nenhuma tag reconhecida) | SKIP | Tipo nao classificado — requer tag explicita |

## Filtros

- **Tamanho maximo**: 1 MB (configuravel via `PKS_IMPORT_MAX_SIZE`)
- **Tipos permitidos**: todos os 5 tipos listados acima na politica default
- **Conteudo sem tag reconhecida**: rejeitado (ContentType::Unknown)
- **Conteudo acima do limite de tamanho**: rejeitado independente do tipo

## Configuracao via Env Vars

| Variavel | Default | Descricao |
|----------|---------|-----------|
| `PKS_IMPORT_MAX_SIZE` | `1048576` (1 MB) | Tamanho maximo em bytes para importacao |

Para aumentar o limite para 5 MB:
```bash
export PKS_IMPORT_MAX_SIZE=5242880
```

## Exemplos por Dominio

### 1. Engenharia de Software

| Conteudo | Decisao | Motivo |
|----------|---------|--------|
| ADR sobre escolha de banco de dados | STORE | Raciocinio arquitetural |
| Dump SQL de 50 MB | SKIP | Dado bruto, excede tamanho |
| Resumo AI de code review | STORE | Contexto sintetizado |
| Binario `.wasm` compilado | SKIP | Artefato de build, sem tag |
| Runbook de deploy para producao | STORE | Procedimento operacional |

### 2. Academico

| Conteudo | Decisao | Motivo |
|----------|---------|--------|
| Notas de aula com reflexoes | STORE | Raciocinio do estudante |
| PDF de artigo cientifico (20 MB) | SKIP | Dado bruto, excede tamanho |
| Resumo AI de capitulo de livro | STORE | Sintese contextual |
| ADR sobre metodologia de pesquisa | STORE | Decisao metodologica |
| Planilha de dados experimentais | SKIP | Dado bruto, sem tag |

### 3. Pessoal

| Conteudo | Decisao | Motivo |
|----------|---------|--------|
| Nota manual sobre metas do ano | STORE | Reflexao pessoal |
| Foto de viagem (5 MB) | SKIP | Binario, excede tamanho |
| Resumo AI de habitos semanais | STORE | Contexto sintetizado |
| Tracker de leituras completadas | STORE | Registro de progresso |
| Backup de conversas (100 MB) | SKIP | Dado bruto, excede tamanho |

### 4. Open-Source

| Conteudo | Decisao | Motivo |
|----------|---------|--------|
| ADR sobre licenciamento MIT vs Apache | STORE | Decisao de projeto |
| Release binary (30 MB) | SKIP | Artefato de build |
| Runbook de triagem de issues | STORE | Procedimento da comunidade |
| Nota sobre RFC proposta | STORE | Contexto de design |
| Log de CI completo | SKIP | Dado efemero, sem tag |

### 5. Pesquisa

| Conteudo | Decisao | Motivo |
|----------|---------|--------|
| ADR sobre escolha de framework ML | STORE | Decisao tecnica |
| Dataset CSV (500 MB) | SKIP | Dado bruto, excede tamanho |
| Resumo AI de experimento | STORE | Raciocinio sintetizado |
| Runbook de reproducao de resultados | STORE | Procedimento cientifico |
| Modelo treinado `.pkl` (200 MB) | SKIP | Artefato binario |
