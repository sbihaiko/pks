---
trigger: always_on
---

# Constituição
Versão: 1.7.0

## Missão e Escopo
Esta Constituição assegura que o projeto permaneça funcional e com alta qualidade.

## Princípios Fundamentais

### Disciplina de Tamanho de Arquivo (NÃO NEGOCIÁVEL)
Mantenha cada arquivo fonte com 500 linhas ou menos (excluindo artefatos gerados e documentação). Se uma mudança arriscar quebrar o limite, divida o arquivo antes da fusão (merge) para que a propriedade, revisões e paridade headless permanecem gerenciáveis. A divisão não deve ser "mecânica", mas sim baseada na intenção buscando a clareza e a simplicidade.

### Abstração Primeiro (NÃO NEGOCIÁVEL)
Prefira estender ou criar abstrações para facilitar a reutilização e a manutenção do código. A composição deve ser priorizada sobre a herança.

### Comunicação Intencional (NÃO NEGOCIÁVEL)
O código deve comunicar sua intenção de forma clara e direta por meio de nomes semânticos precisos, estrutura expressiva e organização coerente. Sempre que a intenção não estiver imediatamente evidente, a resposta correta é renomear, refatorar ou simplificar o código até que ele se torne autoexplicativo. Comentários não devem ser usados para explicar o que o código faz. O uso de comentários é um último recurso e deve ocorrer apenas quando a motivação da decisão não pode ser expressa adequadamente no próprio código, como em casos de trade-offs relevantes, restrições externas ou decisões arquiteturais não óbvias. Quando inevitáveis, comentários devem explicar exclusivamente por que a decisão foi tomada, nunca como o código funciona.

### Clareza de Fluxo de Controle (NÃO NEGOCIÁVEL)
Use cláusulas de guarda (guard clauses) para sair de caminhos inválidos imediatamente, garantindo que o caminho feliz permaneça linear e legível. Cadeias de 'else' e ternárias aninhadas são proibidas; retornos antecipados reduzem a área de superfície que o QA deve validar. Blocos de 'try/catch' aninhados também são proibidos.

Quando diferentes exceções precisam de tratamentos distintos, estratifique os catch em um único try, do mais específico para o mais genérico, em vez de aninhar blocos. Isso mantém o fluxo explícito, evita múltiplos caminhos ocultos de execução e preserva a legibilidade do código. Aninhar try/catch cria dependências implícitas, dificulta o raciocínio, mascara erros reais e torna o comportamento do sistema mais difícil de prever e testar.

Exceções devem ser tratadas no nível correto de responsabilidade. Se a função não sabe resolver o erro de forma clara, ela deve falhar rápido e propagar a exceção.

#### Estratégias por Linguagem

**Python** — Usar múltiplos blocos `except` tipados, do mais específico ao mais genérico:
```python
try:
    result = service.execute()
except HttpError as e:
    if e.resp.status == 404: return handle_not_found()
    if e.resp.status == 403: return handle_forbidden()
    raise
except json.JSONDecodeError:
    return handle_parse_error()
except Exception:
    raise
```

**JavaScript** — Usar guard clauses com `instanceof` em um único catch (a linguagem não suporta múltiplos catch tipados):
```javascript
try {
    return riskyOperation();
} catch (e) {
    if (e instanceof SyntaxError) return handleSyntax(e);
    if (e instanceof TypeError) return handleType(e);
    throw e;
}
```

Para fallbacks sequenciais em JavaScript, extrair para funções helper que retornam objetos `{ success, value, error }` e usar early returns.

#### Anti-Padrão: Refatoração Mecânica

Substituir `else` por `if not condition` **NÃO** resolve o problema — é refatoração mecânica que não melhora a legibilidade:

```python
# ❌ ERRADO: Refatoração mecânica
if condition:
    do_a()
if not condition:
    do_b()

# ✅ CORRETO: Extrair método com early return e nomenclatura clara
def process(data):
    if should_skip(data):
        return handle_skip()
    return handle_main_flow(data)
```

A solução real é: **extrair métodos**, **usar nomenclatura semântica** e **aplicar early returns**.

Quando não for possível usar `return` diretamente (por exemplo, em um bloco dentro de um loop ou função maior), **extrair a lógica para um método separado** que possa usar `return`. Isso transforma código complexo em chamadas a funções com nomes que comunicam a intenção.

### Pegada Mínima de Documentação (NÃO NEGOCIÁVEL)
Vamos trabalhar com poucos arquivos de documentação. 
O README.md deve conter informações gerais sobre o projeto, como como instalar e executar o projeto. 
O JOURNAL.md deve conter um resumo de todas as decisões importantes que tivemos.
O INIT.md deve conter um contexto inicial para que as LLM's possam entender o contexto do projeto e como ele funciona. Mantenha esses arquivos atualizados. Arquivos auxiliares (ex: md, json, scripts) devem ser explicitamente referenciados no README.md para facilitar a navegabilidade.

### Disciplina de Caminho Relativo (NÃO NEGOCIÁVEL)
Cada referência a um arquivo do repositório DEVE usar um caminho relativo à raiz do repositório. Caminhos absolutos são permitidos apenas para recursos fora deste checkout e DEVEM incluir uma justificativa onde quer que sejam registrados.

### Disciplina de Idioma e Localização (NÃO NEGOCIÁVEL)
- **Documentação e Artefatos de Governança**: DEVEM ser redigidos em PT-BR (ex: esta Constituição, README.md, JOURNAL.md).
- **Código-fonte**: Identificadores (variáveis, funções, classes) e mensagens de log/erro DEVEM ser em EN-US para manter compatibilidade e legibilidade internacional.
- **Mensagens de Commit**: DEVEM seguir o padrão Conventional Commits em PT-BR.

### Disciplina de Privilégio Mínimo (NÃO NEGOCIÁVEL)
Operar sempre com o menor nível de permissão necessário para proteger o usuário e garantir segurança arquitetural.
- **Escopos Reduzidos**: O Add-on do Google Workspace deve solicitar apenas acesso ao arquivo atual e arquivos criados por ele (`.../auth/drive.file`), abandonando o acesso total ao Drive (`.../auth/drive`).
- **Arquitetura Service-to-Service**: Delegar operações de visão sistêmica (criação de pastas, buscas cruzadas) para o Backend Python via Conta de Serviço. O Add-on atua apenas como interface de visualização e gatilho (cliente magro).
- **Segurança e Trust UX**: Minimizar a superfície de ataque e aumentar a confiança do usuário, garantindo que o app acesse apenas o estritamente necessário para a tarefa atual.

### Priorização Incremental (NÃO NEGOCIÁVEL)
**Primeiro faz funcionar, depois faz ficar rápido, depois faz ficar bonito.**

Este mantra define a ordem obrigatória de prioridades: Funciona → Rápido → Bonito. O desenvolvimento deve avançar de forma incremental, começando por uma base funcional correta e confiável. Com a funcionalidade validada, o foco passa a ser performance, eficiência e escala. Somente após isso a experiência visual, usabilidade e acabamento devem ser refinados.

Qualquer tentativa de antecipar otimizações ou polimento estético antes de consolidar uma base funcional sólida caracteriza violação deste princípio.

## Guardrails Operacionais
1. **Verificação de Caminhos Relativos** — Garanta conformidade com o princípio de **Disciplina de Caminho Relativo**. Caminhos absolutos são vetados exceto quando justificados para recursos externos.

2. **Cota Zero da Service Account** — A Service Account do GCP (`159418546083-compute@developer.gserviceaccount.com`) possui ZERO cota de armazenamento no Google Drive pessoal. Impossível criar arquivos novos; Shared Drive é obrigatório para cache dinâmico; placeholders necessários para arquivos em pastas locais.

3. **Validação de IDs e Chaves Longas** — IDs de arquivos, chaves de API, hashes e qualquer string longa extraída de imagens ou screenshots DEVEM ser confirmados por texto copiável antes de serem usados no código. Caracteres similares (`I/l`, `0/O`, `1/l`) causam erros silenciosos. Sempre solicitar URL ou texto copiável.

4. **Disciplina de Deploy (STG → Release → PROD)** — Deploys DEVEM seguir o fluxo obrigatório:
   - **Primeiro**: Deploy em Staging (`stg-octo-v2`) via `deploy.sh` ou `./backend-api/deploy.sh STG`.
   - **Segundo**: Validar funcionalidade em STG com testes headless e/ou manuais.
   - **Terceiro**: Executar `release.sh` para criar uma tag Git anotada com changelog.
   - **Quarto**: Deploy em Produção (`prod-octo-v2`) via `prod.sh` usando a tag criada.
   
   Hotfixes emergenciais que violem este fluxo DEVEM ser documentados no JOURNAL.md com justificativa e DEVEM ter uma tag retroativa criada imediatamente após a correção.

5. **Ineexistência de Pasta 'Insights do Gemini'** — "Insights do Gemini" é um recurso de visualização da UI do Google Drive/Workspace e **NÃO É UMA PASTA FÍSICA**. O código NUNCA deve tentar buscar, criar ou listar arquivos dentro de uma pasta com este nome. Arquivos vistos nesta seção estão fisicamente em outros locais (geralmente raiz do usuário ou pasta de dados).

6. **Integração Contínua Local Obrigatória (CI Dashboard)** — Cada commit afeta a saúde do projeto e DEVE obrigatoriamente acionar a skill `ci-dashboard` (via `pre-commit` default em `.git/hooks/pre-commit`). Isso garante que a bateria de testes, cobertura e verificações de risco rodem a cada mudança, mantendo as métricas de saúde perfeitamente sincronizadas ao código submetido. Ignorar este hook só é permitido em extrema urgência.

## Governança
- **Autoridade** — Esta Constituição substitui hábitos informais. Revisões de código, scripts de sincronização e aprovações de implantação DEVEM citar conformidade explicitamente.
- **Política de Versionamento** — Aplique Versionamento Semântico à Constituição: MAJOR para remoções ou redefinições de princípios, MINOR para novos princípios/seções, PATCH para esclarecimentos.
- **Localização dos Artefatos** — A Constituição deve residir em `.agent/rules/constitution.md` para ser compatível com os triggers do Antigravity.
- **Revisão de Conformidade** — Cada revisão de funcionalidade inclui uma Verificação da Constituição referenciando modelos atuais e pacotes de evidência. Auditorias trimestrais verificam se os limites de linguagem, paridade de interface do usuário e documentação de regressão permanecem alinhados com estes princípios.