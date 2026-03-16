
import os
import json
import subprocess
import glob
from concurrent.futures import ThreadPoolExecutor

CONFIG_FILE = "ci-dashboard/config.json"

def load_config():
    with open(CONFIG_FILE, "r") as f:
        return json.load(f)

def run_test(test_file):
    try:
        # Check if file has "repro" in path, maybe skip? No, let's run specs.
        # Filter only .spec.js
        if not test_file.endswith(".spec.js"):
            return None
            
        print(f"Running {test_file}...")
        result = subprocess.run(
            ["node", test_file],
            capture_output=True,
            text=True,
            timeout=30 # 30s timeout per test
        )
        
        outcome = "passed" if result.returncode == 0 else "failed"
        return {
            "nodeid": test_file,
            "outcome": outcome,
            "stdout": result.stdout + "\n" + result.stderr
        }
    except subprocess.TimeoutExpired:
        return {
            "nodeid": test_file,
            "outcome": "failed",
            "stdout": "TIMEOUT (30s)"
        }
    except Exception as e:
        return {
            "nodeid": test_file,
            "outcome": "failed",
            "stdout": str(e)
        }

def scan_tests(test_dirs):
    tests = []
    for d in test_dirs:
        # Encontrar recursivamente em cada diretorio configurado
        tests.extend(glob.glob(os.path.join(d, "**/*.spec.js"), recursive=True))
    return tests

def main():
    config = load_config()
    test_dirs = config.get("test_directories", {}).get("frontend", ["tests/addon"])
    output_file = config.get("output_files", {}).get("frontend", "ci-dashboard/tmp/frontend_results.json")
    max_workers = config.get("max_workers", 4)
    
    test_files = scan_tests(test_dirs)
    print(f"Found {len(test_files)} frontend spec files in {test_dirs}.")
    
    results = []
    results = []
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        futures = [executor.submit(run_test, f) for f in test_files]
        for future in futures:
            res = future.result()
            if res:
                results.append(res)
    
    output_data = {"tests": results}
    
    os.makedirs(os.path.dirname(output_file), exist_ok=True)
    with open(output_file, "w") as f:
        json.dump(output_data, f, indent=2)
        
    print(f"Results saved to {output_file} ({len(results)} tests)")

if __name__ == "__main__":
    main()
