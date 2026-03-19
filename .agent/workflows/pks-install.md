---
description: pks-install
---

# Instalação Automática do PKS (Prometheus Knowledge System)

Este workflow garante a instalação de todas as dependências do PKS de forma limpa em Mac e Windows. O Antigravity vai varrer as ferramentas instaladas, instalar as faltantes de modo desacompanhado e baixar os modelos locais do Ollama.

**Instruções para o Antigravity:**
Use o seu tool de `run_command` para determinar o SO rodando `uname` no terminal atual (macOS/Linux) ou `Get-WmiObject` no caso de PowerShell. A partir do resultado, selecione a suite apropriada abaixo e execute. Todos os passos usam diretivas turbo.

// turbo-all

## Para Mac (macOS):
```bash
# Verifica Homebrew
if ! command -v brew &> /dev/null; then
    echo "Instalando Homebrew..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
fi

# Instala dependências apenas se não existirem
if ! command -v git &> /dev/null; then brew install git; fi
if ! command -v gh &> /dev/null; then brew install gh; fi
if ! command -v ollama &> /dev/null; then brew install --cask --quiet ollama; fi

# Verifica Rust e Cargo
if ! command -v cargo &> /dev/null; then
    echo "Instalando Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Inicia o Ollama e Baixa o modelo
pgrep -f "ollama serve" || OLLAMA_HOST=127.0.0.1:11434 nohup ollama serve > /dev/null 2>&1 &

echo "Aguardando Ollama iniciar..."
for i in {1..30}; do
    if curl -s http://127.0.0.1:11434 > /dev/null; then
        break
    fi
    sleep 1
done

echo "Fazendo pull do modelo vetorial nomic-embed-text..."
ollama pull nomic-embed-text

# Compila e Instala o PKS globalmente
echo "Compilando e instalando PKS..."
if [ -d "pks" ]; then
    cd pks && cargo install --path .
else
    echo "Aviso: Diretório 'pks' não encontrado no path atual. Baixe o código fonte primeiro."
fi
```

## Para Windows (PowerShell):
```powershell
# Verifica e Instala Dependências via Winget (Assumindo Winget nativo no Windows 10/11)

if (-not (Get-Command "git" -ErrorAction SilentlyContinue)) {
    winget install -e --id Git.Git --accept-source-agreements --accept-package-agreements
}

if (-not (Get-Command "gh" -ErrorAction SilentlyContinue)) {
    winget install -e --id GitHub.cli --accept-source-agreements --accept-package-agreements
}

if (-not (Get-Command "ollama" -ErrorAction SilentlyContinue)) {
    winget install -e --id Ollama.Ollama --accept-source-agreements --accept-package-agreements
}

if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    winget install -e --id Rustlang.Rustup --accept-source-agreements --accept-package-agreements
}

$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# Tenta iniciar o Ollama em Background
if (-not (Get-Process "ollama" -ErrorAction SilentlyContinue)) {
    Start-Process "ollama" -WindowStyle Hidden
}

Write-Host "Aguardando Ollama iniciar..."
$retryCount = 0
while ($retryCount -lt 30) {
    try {
        $response = Invoke-RestMethod -Uri "http://127.0.0.1:11434" -Method Get -ErrorAction Stop
        break
    } catch {
        Start-Sleep -Seconds 1
        $retryCount++
    }
}

# Puxando o nomic-embed-text
Write-Host "Fazendo pull do modelo vetorial nomic-embed-text..."
ollama pull nomic-embed-text

# Compila e Instala o PKS globalmente
Write-Host "Compilando e instalando PKS..."
if (Test-Path "pks") {
    Push-Location pks
    cargo install --path .
    Pop-Location
} else {
    Write-Host "Aviso: Diretório 'pks' não encontrado no path atual. Baixe o código fonte primeiro." -ForegroundColor Yellow
}
```

## Verificação Pós-Instalação:
```bash
# Pós instalação (Rode em ambos OS, abstraindo o cmd shell)
rustc --version
git --version
gh --version
ollama list
```

## Configuração da IDE (Antigravity/VSCode)

Após a instalação, o PKS deve ser configurado no arquivo `.mcp.json` para operação via **stdio**. Isso evita a necessidade de gerenciar daemons HTTP/SSE externos.

**Exemplo de configuração no `.mcp.json`:**
```json
{
  "mcpServers": {
    "pks": {
      "type": "stdio",
      "command": "/Users/<seu-usuario>/.../pks/target/release/pks",
      "args": ["--stdio"],
      "env": {
        "PKS_VAULTS_DIR": "/Users/<seu-usuario>/VSCodeProjects"
      }
    }
  }
}
```
