---
description: Instalar o MCP context-mode e configurar as regras de sandbox no Antigravity
---

Este workflow automatiza a instalação do MCP `context-mode` na configuração global do Antigravity e atualiza as regras do projeto (`GEMINI.md` e `global.md`) para garantir o uso automático do sandbox.

### 1. Instalação do MCP Server
Adicione a configuração do `context-mode` no arquivo de configuração do Antigravity.

// turbo
```bash
# O script abaixo adiciona o context-mode ao mcp_config.json se ele não existir
python3 -c "
import json, os
config_path = os.path.expanduser('~/.gemini/antigravity/mcp_config.json')
with open(config_path, 'r') as f: config = json.load(f)
if 'context-mode' not in config.get('mcpServers', {}):
    config.setdefault('mcpServers', {})['context-mode'] = {
        'command': 'npx',
        'args': ['-y', 'context-mode@latest'],
        'env': {},
        'disabled': False
    }
    with open(config_path, 'w') as f: json.dump(config, f, indent=2)
    print('✅ context-mode adicionado ao mcp_config.json')
else:
    print('ℹ️ context-mode já existe na configuração')
"
```

### 2. Atualização das Regras do Projeto (GEMINI.md)
Adicione as diretrizes de execução obrigatória via sandbox.

// turbo
```bash
# Verifica se GEMINI.md existe e adiciona as regras de Context Mode
FILE=".agent/rules/GEMINI.md"
if [ -f "$FILE" ]; then
    if ! grep -q "Context Mode" "$FILE"; then
        echo -e "\n### 🚀 Context Mode & Sandbox Execution (MANDATORY)\n\nPara economizar tokens e manter a continuidade da sessão, você DEVE priorizar o uso das ferramentas do **\`context-mode\`**:\n\n1. **Execução de Código**: Use \`ctx_execute\` para rodar scripts em Python/JS/Shell. Isso evita que logs gigantes entrem no chat.\n2. **Arquivos Grandes**: Se precisar ler arquivos >5KB ou analisar logs, use \`ctx_execute_file\`.\n3. **Documentação e Web**: Use \`ctx_fetch_and_index\` para processar URLs e \`ctx_search\` para buscar apenas o que importa.\n4. **Otimização**: Use \`ctx_batch_execute\` para rodar múltiplos comandos e pesquisas em uma única chamada de ferramenta.\n5. **Diagnóstico**: Se sentir que o ambiente está instável, rode \`ctx_doctor\`." >> "$FILE"
        echo "✅ GEMINI.md atualizado"
    fi
fi
```

### 3. Atualização das Regras Globais (global.md)
Adicione as diretrizes de otimização de contexto e recomendações de performance.

// turbo
```bash
# Verifica se global.md existe e adiciona as regras
FILE=".agent/rules/global.md"
if [ -f "$FILE" ]; then
    if ! grep -q "Otimização de Contexto" "$FILE"; then
        echo -e "\n4. **Otimização de Contexto**: Para QUALQUER operação que envolva leitura de logs, análise de bases de dados ou processamento de arquivos >5KB, utilize mandatoriamente o **\`context-mode\`** (\`ctx_execute\`, \`ctx_execute_file\`, etc.) para evitar poluição da janela de contexto.\n5. **Performance de Sandbox**: Recomenda-se o uso do runtime **Bun** (\`bun\`) para execução de scripts via \`ctx_execute\`. O Bun oferece um ganho de performance de até 5x em comparação com o Node.js puro, o que acelera significativamente as operações de análise programática." >> "$FILE"
        echo "✅ global.md atualizado"
    fi
fi
```

### 4. Verificação Final
Peça para a IA rodar o diagnóstico para garantir que o ambiente está pronto.

```bash
# Dentro do chat do Antigravity:
"Rode o ctx_doctor do context-mode"
```
