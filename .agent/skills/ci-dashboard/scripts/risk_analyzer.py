import os
import re
import json
import argparse

def analyze_file(filepath):
    risks = []
    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
        lines = f.readlines()
        
    line_count = len(lines)
    content = "".join(lines)
    
    # Rule 1: File size > 500 lines
    if line_count > 500:
        risks.append({
            "rule": "Tamanho de Arquivo",
            "severity": "🔴 Crítico",
            "message": f"Arquivo possui {line_count} linhas (limite: 500).",
            "type": "size",
            "value": line_count
        })
        
    # Rule 2: Absolute paths
    abs_paths = re.findall(r'/[Uu]sers/[^/\s]+/[^/\s]+', content)
    if abs_paths:
        display_paths = list(set(abs_paths))[:3]
        risks.append({
            "rule": "Caminhos Relativos",
            "severity": "🟡 Alto",
            "message": f"Caminhos absolutos detectados: {', '.join(display_paths)}",
            "type": "security",
            "value": len(abs_paths)
        })
        
    # Rule 3: Portuguese variables/functions (simple regex)
    # This is a very basic check for common PT-BR suffixes or words
    pt_patterns = [r'_service_pt', r'lista_', r'erro_'] 
    for pattern in pt_patterns:
        if re.search(pattern, content):
            risks.append({
                "rule": "Idioma em Código",
                "severity": "🟡 Alto",
                "message": "Nomenclatura em PT-BR detectada.",
                "type": "convention",
                "value": 1
            })
            break

    # Complexity estimation: number of functions, classes, and control structures
    complexity = len(re.findall(r'\b(def|class|function|if|elif|else|for|while|try|except|catch|with|switch|case)\b', content))
    
    return {
        "risks": risks,
        "metrics": {
            "lines": line_count,
            "complexity": complexity
        }
    }

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("path", help="Path to analyze")
    parser.add_argument("--output", default="ci-dashboard/tmp/risks.json")
    parser.add_argument("--config", default="ci-dashboard/config.json")
    args = parser.parse_args()
    
    # Load exclusions from config
    exclusions = [".git", "__pycache__", ".agent", "ci-dashboard", "node_modules", ".venv", "dist", "coverage", "testsprite_tests", "tests", ".worktrees", "target"]
    if os.path.exists(args.config):
        try:
            with open(args.config, 'r') as f:
                config = json.load(f)
                config_exclusions = config.get("exclude_patterns", [])
                if isinstance(config_exclusions, list):
                    exclusions.extend(config_exclusions)
        except Exception:
            pass
    
    analysis_results = {}
    
    for root, _, files in os.walk(args.path):
        if any(exc in root for exc in exclusions):
            continue
        for file in files:
            if file.endswith(('.py', '.js', '.gs', '.rs')):
                # Skip test files by name pattern
                if any(t_ind in file.lower() for t_ind in ["test_", "_test", ".spec.", ".test."]) or file.endswith("_tests.rs"):
                    continue
                path = os.path.join(root, file)
                result = analyze_file(path)
                if result["risks"] or result["metrics"]["lines"] > 0:
                    analysis_results[os.path.relpath(path, args.path)] = result
                    
    with open(args.output, 'w') as f:
        json.dump(analysis_results, f, indent=2)
    print(f"Risk analysis saved to {args.output}")

if __name__ == "__main__":
    main()
