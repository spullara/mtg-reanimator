#\!/bin/bash
# Run from project root where cards.json lives
cd "$(dirname "$0")/.."
echo "=== Performance Benchmark ==="
echo "Running 100,000 games with seed=42..."
echo ""
time java -jar java/target/mtg-reanimator-1.0-SNAPSHOT.jar run -n 100000 -s 42 2>&1 | tail -5
