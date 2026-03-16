
#!/bin/bash
#echo "🚀 Checking update ..."
#~/.local/bin/claude update
#
echo "🚀 Starting claude code irrestrict..."
~/.local/bin/claude --dangerously-skip-permissions \
    $@
