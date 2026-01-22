#\!/bin/bash
# Run from project root where cards.json lives
cd "$(dirname "$0")/.."

EXPECTED="cada003e38a3739c9a5cc6d9d0d4d8da68e189c77205301dbfc77d01de17639c"

echo "Building..."
cd java && mvn package -q -DskipTests
cd ..

echo "Running deterministic test (seed=42, n=10)..."
ACTUAL=$(java -jar java/target/mtg-reanimator-1.0-SNAPSHOT.jar run -s 42 -v -n 10 2>&1 | grep -v "completed\|games/sec" | shasum -a 256 | cut -d' ' -f1)

if [ "$EXPECTED" = "$ACTUAL" ]; then
    echo "✅ PASS: Hash matches - behavior unchanged"
    echo "Hash: $ACTUAL"
    exit 0
else
    echo "❌ FAIL: Hash mismatch\!"
    echo "Expected: $EXPECTED"
    echo "Actual:   $ACTUAL"
    exit 1
fi
