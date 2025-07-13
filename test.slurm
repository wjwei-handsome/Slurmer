#!/bin/bash

## slurm options
#SBATCH -p test,testf,test-intel         # intel-sc3 partition
#SBATCH -q normal             # normal qos
#SBATCH -J test1              # job name


# This is a dummy shell script for display testing purposes only.

set -euo pipefail

sleep 100s
# Define a very long variable
VERY_LONG_VARIABLE_NAME_WITH_MANY_PARTS_AND_MEANINGLESS_CONTENT="This is a very long value used for testing line wrapping behavior in TUI apps"

# Print environment info
echo "User: $USER"
echo "Current Directory: $(pwd)"
echo "Shell: $SHELL"

# Define a meaningless function
do_nothing_but_look_fancy() {
    for i in {1..5}; do
        echo "Iteration number $i: pretending to do work..."
        sleep 0.1  # simulate doing something
    done
}

# Call the function
do_nothing_but_look_fancy

# Simulate long command line
echo "This is a very very very very very very very very very very very very very very very very very very very very very very long line to test horizontal scrolling or wrapping."

# Some fake logic
if [[ "$RANDOM" -gt 10000 ]]; then
    echo "Random number is large: $RANDOM"
else
    echo "Random number is small: $RANDOM"
fi

# Use of here-document (heredoc)
cat <<EOF
This is a heredoc block.
It contains multiple lines.
Just for testing.
EOF

# End of script
echo "Finished dummy script. Nothing actually happened."
