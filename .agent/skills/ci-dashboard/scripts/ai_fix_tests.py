import os
import re

def parse_tofix():
    if not os.path.exists("TOFIX.md"):
        print("🎉 No TOFIX.md found. Everything looks green!")
        return []
    
    with open("TOFIX.md", "r") as f:
        content = f.read()
    
    # Simple regex to find failed tests
    # Format: ### ❌ [Layer] nodeid
    matches = re.findall(r"### ❌ \[(Backend|Frontend)\] (.*?)\n", content)
    
    tests = []
    for layer, nodeid in matches:
        tests.append({
            "layer": layer,
            "nodeid": nodeid
        })
    
    return tests

if __name__ == "__main__":
    tests = parse_tofix()
    if tests:
        print(f"🔍 Found {len(tests)} tests to fix:")
        for t in tests:
            print(f" - [{t['layer']}] {t['nodeid']}")
        print("\n🤖 AI is ready to initiate fixes.")
    else:
        print("✅ No failing tests detected.")
