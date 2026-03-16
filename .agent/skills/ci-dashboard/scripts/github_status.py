import subprocess
import json
import argparse
from datetime import datetime, timedelta, timezone

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default="ci-dashboard/tmp/ci.json")
    args = parser.parse_args()
    
    try:
        # Get recent workflow runs
        print("Fetching workflow runs from GitHub...")
        result = subprocess.run(
            ["gh", "run", "list", "--limit", "50", "--json", "status,conclusion,displayTitle,headBranch,createdAt,updatedAt,url"],
            capture_output=True, text=True
        )
        if result.returncode == 0:
            raw_runs = json.loads(result.stdout)
            print(f"Found {len(raw_runs)} raw runs.")
            
            # Get current local branch
            try:
                branch_res = subprocess.run(["git", "branch", "--show-current"], capture_output=True, text=True)
                current_branch = branch_res.stdout.strip()
            except:
                current_branch = "main"

            # Filter: Only keep failures if they are recent (e.g., last 24h) OR on the current branch
            now = datetime.now(timezone.utc)
            threshold = now - timedelta(hours=24)

            # Deduplication logic: Group by (title, branch)
            # We want to keep the LATEST run for EACH (title, branch).
            # If the latest run for a title/branch is a success, the failure is "resolved" for that unit of work.
            dedup_map = {}
            for run in raw_runs:
                key = (run["displayTitle"], run["headBranch"])
                
                updated_at_str = run["updatedAt"].replace("Z", "+00:00")
                updated_at = datetime.fromisoformat(updated_at_str)
                
                if key not in dedup_map or updated_at > dedup_map[key]["updated_at_dt"]:
                    run["updated_at_dt"] = updated_at
                    dedup_map[key] = run

            # Final filtering for report relevance
            final_runs = []
            for run in dedup_map.values():
                is_current_branch = run["headBranch"] == current_branch
                is_recent = run["updated_at_dt"] > threshold
                
                # We hide failures IF:
                # 1. They are NOT recent AND NOT on current branch
                # 2. OR if they are older than a success for the same "title" (already covered by dedup_map logic)
                
                # However, some titles are generic (e.g. "release: vX.Y.Z"). 
                # If "release: v6.0.7" failed once but succeeded later, dedup_map keeps the success.
                # If "release: v6.0.7" failed but now we have "release: v6.0.8" success, 
                # the user might still see the old failure if the title changed.
                
                # To really "clean" corrected errors:
                # If it's a failure, check if it's recent or current branch.
                if run["conclusion"] == "failure":
                    if is_current_branch or is_recent:
                        final_runs.append(run)
                else:
                    # Keep successes for history
                    final_runs.append(run)

            # Convert back to sorted list by createdAt desc
            runs = sorted(final_runs, key=lambda x: x["createdAt"], reverse=True)[:15]
            print(f"Filtered to {len(runs)} relevant runs.")

            for run in runs:
                try:
                    start_str = run["createdAt"].replace("Z", "+00:00")
                    start = datetime.fromisoformat(start_str)
                    
                    end = run["updated_at_dt"]
                    duration = end - start
                    minutes, seconds = divmod(duration.total_seconds(), 60)
                    run["duration_text"] = f"{int(minutes)}m {int(seconds)}s"
                    run["created_at_pretty"] = start.strftime("%d/%m %H:%M")
                    
                    if "updated_at_dt" in run:
                        del run["updated_at_dt"]
                except Exception as e:
                    run["duration_text"] = "N/A"
                    run["created_at_pretty"] = run.get("createdAt", "N/A")

            with open(args.output, 'w') as f:
                json.dump(runs, f, indent=2)
            print(f"✅ GitHub status saved to {args.output}")
        else:
            print(f"Error fetching GitHub status: {result.stderr}")
            with open(args.output, 'w') as f:
                json.dump([], f)
    except FileNotFoundError:
        print("GitHub CLI (gh) not found.")
        with open(args.output, 'w') as f:
            json.dump([], f)

if __name__ == "__main__":
    main()
